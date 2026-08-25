---
schema: egohygiene.architecture-decision/v1
id: ADR-001
title: Validate Repository Intelligence source records
status: proposed
date: 2026-08-25
decision_scope: repository
visibility: public
owners: [egohygiene/egolint]
issue: https://github.com/egohygiene/egolint/issues/24
pull_request: null
related: [egohygiene/hygiene#ADR-002]
supersedes: []
superseded_by: []
affected_repositories: [egohygiene/egolint]
affected_contracts: [egohygiene.architecture-decision/v1]
implementation_status: in_progress
evidence: []
exceptions: []
approval: null
extensions: {}
---

# ADR-001: Validate Repository Intelligence source records

## Context

Source records need deterministic local validation.

## Decision

Use Egolint-owned semantic rules.

## Alternatives considered and rejected

Parsing prose in Relay was rejected because it would duplicate meaning.

## Consequences and tradeoffs

Repositories must declare their adoption state.

## Implementation and evidence links

Issue 24 tracks the implementation.

## Replacement or exit strategy

Publish a new versioned rule catalog if the contract changes.

## Follow-up work

Relay will orchestrate the released validator.
