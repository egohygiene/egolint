---
schema: egohygiene.architecture-decision/v1
id: ADR-002
title: Use a legacy projection
status: superseded
date: 2026-08-24
decision_scope: repository
visibility: public
owners: [egohygiene/egolint]
issue: null
pull_request: null
related: []
supersedes: []
superseded_by: [ADR-003]
affected_repositories: [egohygiene/egolint]
affected_contracts: []
implementation_status: not_applicable
evidence: []
exceptions: []
approval:
  date: 2026-08-25
  by: egohygiene-maintainer
  evidence: https://github.com/egohygiene/egolint/pull/24
extensions: {}
---

# ADR-002: Use a legacy projection

## Context
The earlier shape is retained as history.
## Decision
Retain the record while using its replacement.
## Alternatives considered and rejected
Deletion was rejected because it destroys lineage.
## Consequences and tradeoffs
The index keeps one more historical row.
## Implementation and evidence links
The replacement is ADR-003.
## Replacement or exit strategy
ADR-003 is the accepted replacement.
## Follow-up work
No additional work is known.
