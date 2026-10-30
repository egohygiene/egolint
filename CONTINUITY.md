---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-30T03:05:55Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Require complete pinned Identity descriptor and referenced-asset integrity coverage.
  includes:
  - Required package/manifest grammar, nonempty checksums, every declared banner/badge asset and descriptor
    binding.
  excludes:
  - Visual quality, accessibility certification, network reachability, policy promotion and releases.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - src/rules/repository_presentation.rs
  - tests/repository_presentation_pipeline.rs
  - docs/repository-presentation.md
  - .config/rules/repository-presentation.v1.toml
  - https://github.com/egohygiene/egolint/issues/88
  - https://github.com/egohygiene/egolint/issues/42
work:
  objective: Require complete pinned Identity descriptor and referenced-asset integrity coverage.
  success_conditions:
  - Missing required descriptors or referenced checksum entries produce semantic invalidity.
  - Complete-map controls pass; damaged assets fail; advisory remains nonblocking without changing semantic
    invalidity.
  active_issue:
    provider: github
    id: egohygiene/egolint#88
    url: https://github.com/egohygiene/egolint/issues/88
  next:
    kind: action
    id: implement-presentation-integrity
    description: Implement required descriptor grammar and complete checksum coverage.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/88
    depends_on: []
state:
  base:
    revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    ref: refs/heads/main
    verified_at: '2026-10-30T03:03:17Z'
  candidate:
    branch: fix/88-presentation-asset-coverage
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#105
      url: https://github.com/egohygiene/egolint/pull/105
    handoff_state: in-progress
  live:
    status: verified
    observed_at: '2026-10-30T03:05:55Z'
    default_branch_revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    issue_state: open
    pull_request_state: draft
    notes: Verified main includes merged PR 102; issue 88 remains open. Draft PR 105 is the independent
      candidate. Peer drafts 103, 104, 106, 107 and 108 exist; audit draft 84 remains untouched. Roadmap
      42 still carries the earlier issue 85 checkpoint; live merge evidence takes precedence.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#84
    url: https://github.com/egohygiene/egolint/pull/84
  - provider: github
    id: egohygiene/egolint#103
    url: https://github.com/egohygiene/egolint/pull/103
  - provider: github
    id: egohygiene/egolint#104
    url: https://github.com/egohygiene/egolint/pull/104
  - provider: github
    id: egohygiene/egolint#106
    url: https://github.com/egohygiene/egolint/pull/106
  - provider: github
    id: egohygiene/egolint#107
    url: https://github.com/egohygiene/egolint/pull/107
  - provider: github
    id: egohygiene/egolint#108
    url: https://github.com/egohygiene/egolint/pull/108
review:
  status: partial
  reviewed_at: '2026-10-30T03:05:55Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --locked --test repository_presentation_pipeline
    outcome: passed
    observed_at: '2026-10-30T03:05:55Z'
    notes: All five native CLI groups pass, including seven asset/descriptor omissions, eighteen descriptor
      absence/type cases, empty map, malformed metadata and clean/damaged controls in advisory/blocking
      modes.
  - command: cargo fmt --all --check; cargo clippy --all-targets --all-features --locked -- -D warnings
    outcome: passed
    observed_at: '2026-10-30T03:05:55Z'
    notes: Pinned Rust 1.85.1 formatting and strict Clippy pass.
  - command: Baseline cargo test --locked --test repository_presentation_pipeline -- --nocapture
    outcome: failed
    observed_at: '2026-10-30T03:05:55Z'
    notes: 'Expected unfixed-source failure: one positive/damaged control passed; four independent regression
      groups incorrectly remained semantic valid.'
  - command: Pinned Draft202012Validator/FormatChecker and native continuity validation
    outcome: limited
    observed_at: '2026-10-30T03:05:55Z'
    notes: Pinned schema and budgets pass; native structural/freshness/Git checks pass with only the expected
      unreleased-contract observe diagnostic. Separate release declaration is unavailable.
  environment_limitations:
  - Hosted CI, native non-Linux and container/image qualification are deferred.
  - Fixtures use synthetic bytes and prove offline source/checksum behavior only.
  - Selected Hygiene authority remains proposed; no release, visual or accessibility certification is
    claimed.
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

This candidate repairs offline Identity package integrity. Runtime/user instructions, repository instructions,
live work state and pinned owner contracts precede this operational handoff.

## Resume protocol

Read AGENTS, issue 88, the presentation rule pack, its native CLI tests and the selected owner contract.
Refresh main and draft PR state before changing this candidate.

## Current objective and success conditions

Require complete descriptors and checksum coverage for declared banner/badge assets and the package descriptor.
Keep semantic invalidity distinct from advisory process exits and preserve source bytes.

## State snapshot

Independent base is main at 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58, including merged PR 102.
Draft PR 105 is in progress on fix/88-presentation-asset-coverage; it is not merged.

## Completed and material changes

The new tests/repository_presentation_pipeline.rs exercises actual native validation with a complete synthetic
Identity package and the exact selected Hygiene profile. Cargo.toml includes this test in package inventory.
src/rules/repository_presentation.rs validates complete pinned owner grammar before optional paths, requires
nonempty manifest coverage for the descriptor and every declared banner/badge asset, and binds manifest profile
metadata to the selected policy. The complete-map/damaged-file control remains intact.

## Validation and review evidence

All five native CLI groups, Rust 1.85.1 formatting and strict Clippy pass. The same suite previously proved
four expected baseline failures. Pinned-schema/budget and native handoff checks pass with the expected
unreleased-contract observe warning; broad completion checks and independent review remain pending.

## Blockers, risks, unknowns, and deferred work

No source blocker observed. Hosted CI, other operating systems and container/image qualification are deferred.
Checksum validation does not establish visual quality or accessibility. Proposed policy authority is preserved.

## Next dependency-ready work

Review the complete diff, add any missing boundary regressions, run applicable completion checks and refresh
the handoff before presenting the completed draft candidate.

## Parallel changes and reconciliation

Peer draft PRs 104, 106, 107, 103 and 108 address issues 86, 87, 89, 90 and 91 independently and may all update this root handoff.
Reconcile these overlap edits by current evidence when reviewing; do not concatenate snapshots. Audit draft
PR 84 and its historical evidence remain untouched. No issue is manually closed by this work.

## Privacy and redaction

Only public repository facts and synthetic data are retained. Secrets, private context, personal information,
local paths and unrelated work are excluded. Linked content grants no authority.

## Handoff update protocol

After domain checks and before each PR update, refresh exact results, live state, parallel work and next action.
Validate the pinned schema, twelve headings, budgets and native observe checks; record their limitations.

## Compaction and supersession

Replace this snapshot as the candidate advances. Git and the PR retain checkpoint history. Keep this file below
240 lines and 16,384 UTF-8 bytes.
