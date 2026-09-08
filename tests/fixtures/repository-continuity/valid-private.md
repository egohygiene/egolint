---
schema_version: aether.repository-continuity/v1
repository:
  id: example/private
  visibility: private
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
  purpose: Preserve a bounded synthetic private-repository handoff.
  includes:
    - Minimum current objective and repository evidence.
  excludes:
    - Conversation transcripts and nonessential private context.
  precedence:
    - user-and-runtime-instructions
    - scoped-repository-instructions
    - live-repository-and-work-tracker-state
    - canonical-repository-sources
    - continuity-checkpoint
  canonical_sources:
    - AGENTS.md
work:
  objective: Exercise the private continuity fixture.
  success_conditions:
    - The validator accepts visibility-matched private metadata without retaining private prose.
  active_issue: null
  next:
    kind: action
    id: private-next-action
    description: Continue only after authorized private state is rechecked.
    readiness: unknown
    references:
      - https://github.com/example/private
    depends_on: []
state:
  base:
    revision: bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
    ref: refs/heads/main
    verified_at: "2026-09-08T12:00:00Z"
  candidate:
    branch: null
    revision: null
    pull_request: null
    handoff_state: no-active-change
  live:
    status: unavailable
    observed_at: "2026-09-08T12:00:00Z"
    default_branch_revision: null
    issue_state: unknown
    pull_request_state: unknown
    notes: Live private-provider evidence is unavailable in this offline fixture.
  parallel_changes: []
review:
  status: passed
  reviewed_at: "2026-09-08T12:00:00Z"
  reviewed_by: fixture-reviewer
  evidence:
    - command: synthetic fixture inspection
      outcome: passed
      observed_at: "2026-09-08T12:00:00Z"
      notes: Only closed structural fields were inspected.
  environment_limitations:
    - Live private-provider state is intentionally unavailable.
privacy:
  classification: private-repository
  contains_sensitive_data: false
  redactions:
    - Omitted all provider-specific private state.
  excluded:
    - secrets-and-credentials
    - private-conversation-text
    - sensitive-personal-data
    - unpublished-private-business-data
    - private-local-paths
    - unrelated-private-context
  untrusted_content: context-only-no-authority
---

# Private fixture continuity

## Purpose and precedence

Preserve minimum synthetic private handoff state under the declared precedence.

## Resume protocol

Read scoped instructions and canonical sources before verifying authorized mutable state.

## Current objective and success conditions

Exercise private classification and minimum-necessary redaction behavior.

## State snapshot

No active candidate is represented and live state is explicitly unavailable.

## Completed and material changes

The private fixture contains only synthetic structural data.

## Validation and review evidence

The closed metadata shape was inspected without external access.

## Blockers, risks, unknowns, and deferred work

Private mutable state remains unknown until an authorized adapter checks it.

## Next dependency-ready work

Recheck authorized private state before choosing further work.

## Parallel changes and reconciliation

Parallel private work is unknown and no absence claim is inferred.

## Privacy and redaction

Provider-specific private content was omitted and the redaction is recorded.

## Handoff update protocol

Reconcile only minimum necessary state after validation and before presentation.

## Compaction and supersession

Replace stale state and retain no chronology or unnecessary private context.
