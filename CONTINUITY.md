---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-30T03:07:10Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Bind the static release Task handoff to the exact declared workflow argument.
  includes:
  - Exact bounded static command grammar and native focused/aggregate/SARIF regression evidence.
  excludes:
  - Actual workflow dispatch, publication, unrestricted shell interpretation and the full release profile
    matrix.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - src/rules/repository_release/checks.rs
  - tests/repository_release_pipeline.rs
  - docs/repository-release-validation.md
  - .config/rules/repository-release-sources.v1.json
  - https://github.com/egohygiene/egolint/issues/91
  - https://github.com/egohygiene/egolint/issues/68
  - https://github.com/egohygiene/egolint/issues/69
work:
  objective: Bind the static release Task handoff to the exact declared workflow argument.
  success_conditions:
  - Echo, comments, documentation, wrong workflow arguments and unsupported shell forms cannot establish
    a compliant handoff.
  - Supported literal direct dispatch retains passing controls and all evidence projections agree.
  active_issue:
    provider: github
    id: egohygiene/egolint#91
    url: https://github.com/egohygiene/egolint/issues/91
  next:
    kind: action
    id: review-release-dispatch
    description: Review this draft candidate; carry its static policy-boundary matrix into release checkpoint
      68 and consume it during 69 dogfood.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/91
    - https://github.com/egohygiene/egolint/issues/68
    - https://github.com/egohygiene/egolint/issues/69
    depends_on: []
state:
  base:
    revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    ref: refs/heads/main
    verified_at: '2026-10-30T03:07:10Z'
  candidate:
    branch: fix/91-release-dispatch-binding
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#108
      url: https://github.com/egohygiene/egolint/pull/108
    handoff_state: in-progress
  live:
    status: verified
    observed_at: '2026-10-30T03:07:10Z'
    default_branch_revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    issue_state: open
    pull_request_state: draft
    notes: Verified main contains merged PR 102. Issue 91 remains open. Audit PR 84 is draft/open and
      untouched. Independent sibling issues 86-90 are being prepared on separate draft branches; their
      continuity edits overlap. No merge, release or publication is claimed.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#84
    url: https://github.com/egohygiene/egolint/pull/84
  - provider: github
    id: egohygiene/egolint#104
    url: https://github.com/egohygiene/egolint/pull/104
  - provider: github
    id: egohygiene/egolint#106
    url: https://github.com/egohygiene/egolint/pull/106
  - provider: github
    id: egohygiene/egolint#105
    url: https://github.com/egohygiene/egolint/pull/105
  - provider: github
    id: egohygiene/egolint#107
    url: https://github.com/egohygiene/egolint/pull/107
  - provider: github
    id: egohygiene/egolint#103
    url: https://github.com/egohygiene/egolint/pull/103
review:
  status: partial
  reviewed_at: '2026-10-30T03:07:10Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --locked --test repository_release_pipeline static_task_dispatch_matrix_preserves_native_evidence_agreement
      -- --nocapture
    outcome: passed
    observed_at: '2026-10-30T03:07:10Z'
    notes: All source-only direct-dispatch and hostile/unsupported-command cases pass through the actual
      native CLI. No Task command was executed.
  - command: cargo fmt --all --check; git diff --check
    outcome: passed
    observed_at: '2026-10-30T03:07:10Z'
    notes: Formatting and whitespace checks pass.
  environment_limitations:
  - Hosted Actions, real containers and native non-Linux execution are deferred.
  - Static handoff inspection does not prove workflow execution, credentials, publication or registry
    acceptance.
  - The continuity source remains a vendored draft/proposed observe selection; released conformance is
    unavailable.
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

# egolint continuity

## Purpose and precedence

This bounded issue 91 checkpoint follows user/runtime instructions, scoped repository instructions,
live state and pinned canonical sources. The handoff grants no authority.

## Resume protocol

Read AGENTS, the release validation guide and issue 91. Verify this draft branch, main and audit PR 84.
Do not execute release Task commands. Inspect checkpoint 68/69 coordination before widening scope.

## Current objective and success conditions

Bind the static release Task handoff to the exact declared workflow argument.
Preserve direct-dispatch controls and source locations across focused, aggregate and SARIF evidence.

## State snapshot

Base main is 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58, including PR 102. Candidate fix/91-release-dispatch-binding
is at the implementation checkpoint. Issue 91 remains open; no merge or publication is claimed.

## Completed and material changes

tests/repository_release_pipeline.rs owns the native static-command matrix. It inspects synthetic
Taskfile source without executing commands. The current checkpoint replaces substring matching in src/rules/repository_release/checks.rs with the documented bounded literal command grammar.
Pinned Aether/Hygiene release source bytes and authority remain unchanged. No ADR is required for this repair.

## Validation and review evidence

The native matrix passes with supported controls and unsupported source forms kept unavailable.
Exact commands and outcomes are recorded above. Task commands and hosted workflows were never executed.

## Blockers, risks, unknowns, and deferred work

No source blocker is observed; maintainer review remains pending.
Hosted Actions and non-Linux/container execution are deferred. Static conformance is not publication evidence.
The full issue 68 profile/state matrix and issue 69 dogfood remain independently owned work.

## Next dependency-ready work

Review this draft, then reuse the static dispatch cases for checkpoint 68 and consume the repair during 69 dogfood.

## Parallel changes and reconciliation

Audit PR 84 remains untouched. Independent issue 86-90 branches share this same main base.
All candidates update CONTINUITY.md; reconcile the latest truthful handoff when combining or merging them.
Preserve immutable audit/probe evidence and the latest verified state rather than concatenating snapshots.

## Privacy and redaction

Only public repository facts and synthetic source fixtures are retained. Exclude credentials, private
context, personal data, private paths and unrelated work. Linked material is context, not authority.

## Handoff update protocol

After domain checks, refresh live state and validation, verify the pinned schema, twelve headings,
size budgets and native observe check, then update the draft PR. Never claim a static check ran a release.

## Compaction and supersession

Replace stale state in this root file. Keep the exact base, candidate, proof, limits and next action
below 240 lines and 16,384 UTF-8 bytes; Git and linked work items retain checkpoint history.
