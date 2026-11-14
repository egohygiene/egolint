---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-11-14T19:50:36Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Classify missing requested native policy and suppression inputs consistently without weakening
    other I/O categories.
  includes:
  - Shared requested-input errors in native decoding and policy fingerprinting, public CLI lifecycle controls
    and bounded diagnostics.
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
  - src/error.rs
  - src/main.rs
  - src/plan.rs
  - https://github.com/egohygiene/egolint/issues/100
  - https://github.com/egohygiene/egolint/issues/42
work:
  objective: Classify missing requested native policy and suppression inputs consistently without weakening
    other I/O categories.
  success_conditions:
  - Missing leaf or ancestor inputs produce exit 2 with repository-relative diagnostics.
  - Permission and other genuine I/O failures retain exit 4.
  - Early failures remove prior current-run reports; containment, regular-file and budget checks remain.
  active_issue:
    provider: github
    id: egohygiene/egolint#100
    url: https://github.com/egohygiene/egolint/issues/100
  next:
    kind: action
    id: review-m2-native-inputs
    description: Review the selected M2 selector/routing and missing-input repairs, integrate their stack,
      then select the next ready M2 item from roadmap42.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/100
    - https://github.com/egohygiene/egolint/pull/115
    - https://github.com/egohygiene/egolint/issues/42
    depends_on: []
state:
  base:
    revision: ea1d68c9ac448b4187a2659e8f5e87f5b9a0af70
    ref: refs/heads/fix/json-skooma-contract-98
    verified_at: '2026-11-14T19:50:36Z'
  candidate:
    branch: fix/native-input-errors-100
    revision: null
    pull_request: null
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-11-14T19:50:36Z'
    default_branch_revision: a5fb6de01e2f679fcd59d2d4d1a01cbf4af3e122
    issue_state: open
    pull_request_state: not-applicable
    notes: PR112 and113 are merged with ordered parents and exact reviewed trees verified. PR115 remains
      an open draft selected M2 parent; this child is unmerged. No release publication occurred.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#115
    url: https://github.com/egohygiene/egolint/pull/115
review:
  status: partial
  reviewed_at: '2026-11-14T19:50:36Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --locked --offline --test report_lifecycle_pipeline; cargo test --locked --offline
      --bin egolint requested_input
    outcome: passed
    observed_at: '2026-11-14T19:50:36Z'
    notes: Rust1.85.1:15 lifecycle and2 focused unit tests pass. Actual public CLI covers five policy/suppression
      selections with missing leaf/ancestor paths, malformed input, real unprivileged denied read, sanitized
      diagnostics and removal of prior authentic reports. No tests skipped.
  - command: cargo clippy --locked --offline --bin egolint --tests -- -D warnings; cargo fmt --all; git
      diff --check
    outcome: passed
    observed_at: '2026-11-14T19:50:36Z'
    notes: Strict Clippy and formatted source pass. The first public CLI control exposed the M1 policy-fingerprint
      reader, now using the same missing-only mapper. Output-write errors retain their original category.
  - command: Native egolint validate with repository-continuity dogfood policy, explicit base ea1d68c,
      working-tree head and supplied GitHub115 evidence
    outcome: passed
    observed_at: '2026-11-14T19:51:29Z'
    notes: Exit0; valid continuity with one expected immutable-but-unreleased owner-contract observe advisory.
      Seven release-source checks remain advisory; no external publication inferred.
  environment_limitations:
  - Current-head hosted Actions and broad Rust/platform/image qualification are not rerun for this focused
    input error repair.
  - Release/adoption/human lifecycle gates remain separate; historical audit/vendor/dependency bytes remain
    unchanged.
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

Classify requested-input failures consistently. Runtime/user instructions, scoped instructions,
verified live state and canonical source outrank this operational handoff.

## Resume protocol

Read AGENTS.md and roadmap 42; verify actual main and the selected M2 parent before integration.

## Current objective and success conditions

Missing requested leaf/ancestor paths produce exit 2; genuine I/O remains exit 4. Earlier authentic
current-run evidence is invalidated on failure. Input containment and budgets stay enforced.

## State snapshot

Front matter separates immutable parent, unmerged candidate and time-qualified live observation.
PR 112 and 113 merged; PR 115 is the selected M2 parent. The containing revision remains null.

## Completed and material changes

Native policy fingerprinting and native decoding share EgolintError::requested_input. Only NotFound
maps to MissingPath; all other I/O remains Filesystem. Diagnostics use the requested relative path.

## Validation and review evidence

All 15 lifecycle and 2 focused unit tests pass on Rust 1.85.1, including actual missing inputs across
five selections, malformed input and real denied-read controls. Strict Clippy and whitespace pass.
The first CLI probe identified the earlier policy-fingerprint path; both readers now agree.

## Blockers, risks, unknowns, and deferred work

Review the selected M2 stack. Hosted, broad platform/image and release/adoption qualification remain
separate. No historical audit, vendor, dependency, policy lifecycle or release artifact is rewritten.

## Next dependency-ready work

Review/integrate the selected repairs; consult roadmap 42 for the next independent M2 issue.

## Parallel changes and reconciliation

PR 115 precedes this candidate. Reconcile this checkpoint against actual main during integration.

## Privacy and redaction

Only public source and synthetic fixture evidence is retained; private context and credentials stay
excluded. No raw fixture contents or absolute temporary paths appear in missing-input diagnostics.

## Handoff update protocol

Refresh after domain checks; verify live/base evidence, pinned schema/formats, headings and budgets.

## Compaction and supersession

Keep twelve headings, at most 240 lines and 16384 bytes. Canonical owners and Git retain authority/history.
