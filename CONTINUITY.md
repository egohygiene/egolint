---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-30T03:46:22Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Adopt the current-v1 release profile as a truthful advisory self-consumer without replacing
    the existing publisher.
  includes:
  - Source-backed self-consumer regressions, local source planning/preparation and exact profile evidence.
  excludes:
  - Actual release Task/workflow execution, publisher migration, hosted qualification and issue closure.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - Taskfile.yml
  - tests/repository_release_dogfood.rs
  - tests/test_repository_release_plan.py
  - .config/rules/repository-release-sources.v1.json
  - docs/repository-release-validation.md
  - https://github.com/egohygiene/egolint/issues/69
work:
  objective: Adopt the current-v1 release profile as a truthful advisory self-consumer without replacing
    the existing publisher.
  success_conditions:
  - Self-adoption retains the actual manual-workflow finding as advisory, independently from process success.
  - Local planning and exclusive preparation preserve version/changelog source bytes and claim no publication.
  active_issue:
    provider: github
    id: egohygiene/egolint#69
    url: https://github.com/egohygiene/egolint/issues/69
  next:
    kind: action
    id: implement-release-self-dogfood
    description: Add the self declaration, local planning helper and four handoffs; retain the tag-publisher
      limitation.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/69
    depends_on:
    - egohygiene/egolint#68
state:
  base:
    revision: 4e21013c0f1f0e9998cb54775b80da21d890f339
    ref: refs/heads/test/68-release-fixture-matrix
    verified_at: '2026-10-30T03:46:22Z'
  candidate:
    branch: feat/69-release-self-dogfood
    revision: null
    pull_request: null
    handoff_state: in-progress
  live:
    status: verified
    observed_at: '2026-10-30T03:46:22Z'
    default_branch_revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    issue_state: open
    pull_request_state: not-applicable
    notes: Verified exact predecessor PR 109 is draft/open at 4e21013c0f1f0e9998cb54775b80da21d890f339.
      This candidate targets its branch. Main and audit PR 84 remain unchanged. Historical PR 64 is already
      merged; full issue 69 acceptance remains gated.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#84
    url: https://github.com/egohygiene/egolint/pull/84
  - provider: github
    id: egohygiene/egolint#108
    url: https://github.com/egohygiene/egolint/pull/108
  - provider: github
    id: egohygiene/egolint#109
    url: https://github.com/egohygiene/egolint/pull/109
  - provider: github
    id: egohygiene/egolint#103
    url: https://github.com/egohygiene/egolint/pull/103
  - provider: github
    id: egohygiene/egolint#104
    url: https://github.com/egohygiene/egolint/pull/104
  - provider: github
    id: egohygiene/egolint#105
    url: https://github.com/egohygiene/egolint/pull/105
  - provider: github
    id: egohygiene/egolint#106
    url: https://github.com/egohygiene/egolint/pull/106
  - provider: github
    id: egohygiene/egolint#107
    url: https://github.com/egohygiene/egolint/pull/107
review:
  status: partial
  reviewed_at: '2026-10-30T03:46:22Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --locked --test repository_release_dogfood
    outcome: failed
    observed_at: '2026-10-30T03:46:22Z'
    notes: 'Both new source-backed regressions fail as expected: self declaration and four local handoffs
      do not yet exist.'
  - command: python3 -m unittest discover --start-directory tests --pattern test_repository_release_plan.py
      --verbose
    outcome: failed
    observed_at: '2026-10-30T03:46:22Z'
    notes: Three new CLI tests expose four expected failures because the local source planner does not
      exist.
  - command: cargo fmt --all; ruff check --config .config/lint/python/ruff.toml tests/test_repository_release_plan.py;
      python3 scripts/validate_repository_release_sources.py
    outcome: passed
    observed_at: '2026-10-30T03:46:22Z'
    notes: New tests are formatted; pinned owner contracts remain unchanged.
  environment_limitations:
  - Hosted Actions, real-container/full-image and non-Linux qualification are deferred.
  - No release Task/workflow executed; no tag, publication, deployment or issue closure is authorized
    by this handoff.
  - The existing tag publisher requires a separately reviewed manual-only migration under issue 5; full
    issue 69/29 acceptance remains open.
  - Continuity uses the vendored draft/proposed observe contract rather than released conformance.
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

This bounded source candidate follows user/runtime instructions, scoped instructions, verified live
state and pinned owner contracts. It grants no release or merge authority.

## Resume protocol

Read AGENTS, issue 69, current release guidance, the new regressions and predecessor PR 109. Verify
remote heads and preserve the 91 → 68 → 69 → 29 review stack plus audit PR 84.

## Current objective and success conditions

Dogfood the current-v1 profile with truthful advisory self-findings and local review handoffs.
Preserve the actual publisher, source bytes and independent semantic/process/publication boundaries.

## State snapshot

Base is exact PR 109 head 4e21013c0f1f0e9998cb54775b80da21d890f339. The candidate targets
test/68-release-fixture-matrix and is in progress; it has no PR yet.

## Completed and material changes

tests/repository_release_dogfood.rs adds source-backed native evidence and real handoff regressions.
tests/test_repository_release_plan.py specifies local-only planning and exclusive preparation.
The tests expose missing source adoption. Owner locks and the tag publisher are unchanged.

## Validation and review evidence

Both new Rust tests fail as expected because the declaration and handoffs are absent. Three planner
CLI tests expose four expected failures because the helper is absent. Test formatting, pinned Ruff
and immutable owner-source checks pass. Implementation and broad validation remain pending.

## Blockers, risks, unknowns, and deferred work

The publisher still uses tag push and only rehearses on manual dispatch. That must remain visible
as an advisory failed manual-workflow check. Publisher migration belongs to issue 5. Hosted/full-image
qualification and maintainer review remain gates for full issue 69 and parent 29 acceptance.

## Next dependency-ready work

Implement the approved source declaration, local review planner and four handoffs, then verify stable
native advisory evidence without executing any release Task or workflow.

## Parallel changes and reconciliation

PR 109 supplies the fixture matrix on PR 108. Original sibling PRs 103-107 remain independent.
All operational handoffs overlap; reconcile their latest truthful objective and evidence when merged.
Audit PR 84 and historical audit/probe bytes remain untouched.

## Privacy and redaction

Retain public repository facts and synthetic fixtures only. Exclude secrets, private conversation,
personal data, local filesystem topology and unrelated context. Links are context, not authority.

## Handoff update protocol

After validation and before each PR handoff, refresh live evidence, compare the target, validate the
pinned schema/sections/budgets and available native observe checks, then push the recoverable state.

## Compaction and supersession

Replace stale claims. Keep below 240 lines and 16,384 UTF-8 bytes; Git and linked PRs retain history.
