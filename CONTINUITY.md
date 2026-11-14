---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-11-14T19:35:36Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Repair JSONSkooma selector errors and deterministic report routing under M2.
  includes:
  - Typed selector validation, one invocation-directory report anchor and actual CLI routing controls.
  excludes:
  - Release publication, policy authority promotion, fleet rollout and historical audit rewriting.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - docs/json-skooma-evaluation.md
  - docs/release-dependency-handoff.md
  - https://github.com/egohygiene/egolint/issues/98
  - https://github.com/egohygiene/egolint/issues/42
work:
  objective: Repair JSONSkooma selector errors and deterministic report routing under M2.
  success_conditions:
  - Malformed selector shapes emit exit 2 evidence before Ruby starts.
  - All activation branches use the same report destination without consumer mutations.
  - Existing skip and stdout/absolute-output behavior remains supported.
  active_issue:
    provider: github
    id: egohygiene/egolint#98
    url: https://github.com/egohygiene/egolint/issues/98
  next:
    kind: action
    id: review-m2-json-skooma
    description: Review the bounded M2 repair after the existing M1 stack, then continue missing native
      input classification under issue 100.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/98
    - https://github.com/egohygiene/egolint/issues/100
    - https://github.com/egohygiene/egolint/pull/113
    depends_on: []
state:
  base:
    revision: 9c123f82e8918b5064202d5c42a139e4ab3c1781
    ref: refs/heads/codex/egolint-release-dependencies
    verified_at: '2026-11-14T19:35:36Z'
  candidate:
    branch: fix/json-skooma-contract-98
    revision: null
    pull_request: null
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-11-14T19:35:36Z'
    default_branch_revision: f14f7fd3ab0ce9e87c367352fcb3af11a32eda59
    issue_state: open
    pull_request_state: not-applicable
    notes: Verified main f14f7fd3ab0ce9e87c367352fcb3af11a32eda59; PRs 112 and 113 remain the existing
      open M1 stack. This M2 source candidate is unmerged; no hosted or product publication result is
      asserted.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#112
    url: https://github.com/egohygiene/egolint/pull/112
  - provider: github
    id: egohygiene/egolint#113
    url: https://github.com/egohygiene/egolint/pull/113
review:
  status: partial
  reviewed_at: '2026-11-14T19:35:36Z'
  reviewed_by: Codex
  evidence:
  - command: python3 -m unittest discover -s tests -p test_json_skooma_adapter.py -v
    outcome: passed
    observed_at: '2026-11-14T19:35:36Z'
    notes: Ten tests pass, including 21 malformed shape/type controls with no Ruby subprocess, actual
      CLI routing from another directory across five states, stdout/absolute output and unchanged workspace
      bytes. The trusted child is a routing stub, not Ruby schema qualification.
  - command: Ruff 0.16.10 check/format with .config/lint/python/ruff.toml; git diff --check
    outcome: passed
    observed_at: '2026-11-14T19:35:36Z'
    notes: Focused configured Python static checks and whitespace pass. Full historical M1 qualification
      remains attributed to its original source.
  environment_limitations:
  - Current-head hosted Actions and actual pinned Ruby schema semantics are not rerun for this selector/routing
    repair.
  - Existing M1 image/non-Linux, ratchet-baseline, lifecycle and release qualification limits remain in
    docs/release-dependency-handoff.md.
privacy:
  classification: public-repository
  contains_sensitive_data: false
  redactions:
  - Private execution context, credentials, raw logs and private filesystem details omitted.
  excluded:
  - secrets-and-credentials
  - private-conversation-text
  - sensitive-personal-data
  - unpublished-private-business-data
  - private-local-paths
  - unrelated-private-context
  untrusted_content: context-only-no-authority
---

# Egolint continuity

## Purpose and precedence

Repair the bounded M2 selector/routing defects. User/runtime instructions, scoped instructions,
verified live state and canonical sources outrank this operational handoff.

## Resume protocol

Read AGENTS.md and roadmap 42; verify main and the existing M1 stack before integrating this child.

## Current objective and success conditions

Reject malformed selectors before Ruby and use one invocation-directory report destination.

## State snapshot

Front matter separates the immutable base, unmerged candidate and time-qualified provider snapshot.
The containing revision remains null. PR 112 precedes PR 113; this candidate follows that stack.

## Completed and material changes

Schema versions require integer 1 and modes require supported strings. Relative reports are resolved
once before workspace selection; skip, invalid, unavailable, timeout and child branches share them.

## Validation and review evidence

The ten focused tests include malformed-shape/no-subprocess controls and actual CLI routing with a
trusted Python child. Consumer bytes remain unchanged. Configured Ruff and whitespace checks pass.
Actual Ruby schema semantics and older M1 proof remain separately attributed, without retiming.

## Blockers, risks, unknowns, and deferred work

Review/integrate the M1 stack before this child. Hosted/image/non-Linux qualification and publication
remain separate. No policy lifecycle or historical audit evidence is changed.

## Next dependency-ready work

Review this bounded repair and continue issue 100 missing native input classification.

## Parallel changes and reconciliation

PR 112 and PR 113 are the existing M1 stack. Reconcile this checkpoint against the actual merge target.

## Privacy and redaction

Only public source and synthetic controls are retained; private context and credentials are excluded.

## Handoff update protocol

Refresh after domain checks and verify live/base evidence, pinned schema/formats and size budgets.

## Compaction and supersession

Keep twelve headings, at most 240 lines and 16384 bytes. Git and canonical owners retain history.
