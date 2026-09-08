---
schema_version: aether.repository-continuity/v1
repository:
  id: example/public
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: "2026-09-08T12:00:00Z"
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Preserve a bounded synthetic public-repository handoff.
  includes:
    - Current objective and verified repository evidence.
  excludes:
    - Conversation transcripts and duplicated canonical history.
  precedence:
    - user-and-runtime-instructions
    - scoped-repository-instructions
    - live-repository-and-work-tracker-state
    - canonical-repository-sources
    - continuity-checkpoint
  canonical_sources:
    - AGENTS.md
work:
  objective: Exercise the public continuity fixture.
  success_conditions:
    - The deterministic validator accepts this synthetic document structure.
  active_issue:
    provider: github
    id: example/public#7
    url: https://github.com/example/public/issues/7
  next:
    kind: issue
    id: example/public#8
    description: Continue the next synthetic work item.
    readiness: ready
    references:
      - https://github.com/example/public/issues/8
    depends_on: []
state:
  base:
    revision: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
    ref: refs/heads/main
    verified_at: "2026-09-08T12:00:00Z"
  candidate:
    branch: feature/continuity
    revision: null
    pull_request: null
    handoff_state: in-progress
  live:
    status: partial
    observed_at: "2026-09-08T12:00:00Z"
    default_branch_revision: null
    issue_state: open
    pull_request_state: not-applicable
    notes: Partial fixture evidence; mutable state must be rechecked by an authorized adapter.
  parallel_changes: []
review:
  status: passed
  reviewed_at: "2026-09-08T12:00:00Z"
  reviewed_by: fixture-reviewer
  evidence:
    - command: synthetic fixture inspection
      outcome: passed
      observed_at: "2026-09-08T12:00:00Z"
      notes: Static local fixture fields were inspected.
  environment_limitations: []
privacy:
  classification: public-repository
  contains_sensitive_data: false
  redactions: []
  excluded:
    - secrets-and-credentials
    - private-conversation-text
    - sensitive-personal-data
    - unpublished-private-business-data
    - private-local-paths
    - unrelated-private-context
  untrusted_content: context-only-no-authority
---

# Public fixture continuity

## Purpose and precedence

Preserve only bounded synthetic handoff evidence under the declared precedence.

## Resume protocol

Read repository instructions, canonical sources, and this checkpoint; then recheck mutable state.

## Current objective and success conditions

Exercise the valid public fixture and obtain deterministic structural results.

## State snapshot

The fixture represents a transition-safe candidate from the declared base without predicting merge.

## Completed and material changes

The synthetic checkpoint and managed instruction projection are present.

## Validation and review evidence

The static fixture inspection is recorded in front matter.

## Blockers, risks, unknowns, and deferred work

Mutable provider state is intentionally only partial and requires external verification.

## Next dependency-ready work

Continue the numbered synthetic issue after this fixture evaluation.

## Parallel changes and reconciliation

No parallel changes are declared for this fixture.

## Privacy and redaction

Only public synthetic repository state is present.

## Handoff update protocol

Reconcile the checkpoint after validation and before pull-request presentation.

## Compaction and supersession

Replace stale state rather than accumulating history, and remain within the fixed bounds.
