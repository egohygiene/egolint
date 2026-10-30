---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-30T03:01:37Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Reject malformed release image and product contracts before environment consistency checks.
  includes:
  - Reviewed image/product grammar, controlled configuration diagnostics and actual CLI regression controls.
  excludes:
  - Image builds, registry publication, protected promotion and adoption.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - .config/release/base-images.v1.json
  - scripts/validate_release_inputs.py
  - tests/test_release_inputs.py
  - docs/releasing.md
  - https://github.com/egohygiene/egolint/issues/90
  - https://github.com/egohygiene/egolint/issues/42
work:
  objective: Reject malformed release image and product contracts before environment consistency checks.
  success_conditions:
  - Every required image and product record has the complete supported object grammar.
  - Invalid configuration produces controlled nonzero CLI diagnostics before environment comparisons.
  - Valid pinned environment and current CLI dual-platform/full-engine AMD64 controls remain accepted.
  active_issue:
    provider: github
    id: egohygiene/egolint#90
    url: https://github.com/egohygiene/egolint/issues/90
  next:
    kind: action
    id: repair-release-input-grammar
    description: Implement typed grammar checks, then rerun focused CLI regressions.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/90
    depends_on: []
state:
  base:
    revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    ref: refs/heads/main
    verified_at: '2026-10-30T03:01:37Z'
  candidate:
    branch: fix/90-release-input-grammar
    revision: null
    pull_request: null
    handoff_state: in-progress
  live:
    status: verified
    observed_at: '2026-10-30T03:01:37Z'
    default_branch_revision: 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58
    issue_state: open
    pull_request_state: not-applicable
    notes: Main includes merged PR 102. Issue 90 is open with no comments. This candidate has no PR yet.
      Audit PR 84 remains separate and untouched.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#84
    url: https://github.com/egohygiene/egolint/pull/84
  - provider: github
    id: egohygiene/egolint#86
    url: https://github.com/egohygiene/egolint/issues/86
  - provider: github
    id: egohygiene/egolint#87
    url: https://github.com/egohygiene/egolint/issues/87
  - provider: github
    id: egohygiene/egolint#88
    url: https://github.com/egohygiene/egolint/issues/88
  - provider: github
    id: egohygiene/egolint#89
    url: https://github.com/egohygiene/egolint/issues/89
  - provider: github
    id: egohygiene/egolint#91
    url: https://github.com/egohygiene/egolint/issues/91
review:
  status: partial
  reviewed_at: '2026-10-30T03:01:37Z'
  reviewed_by: Codex
  evidence:
  - command: python3 -m unittest discover --start-directory tests --pattern test_release_inputs.py --verbose
    outcome: failed
    observed_at: '2026-10-30T03:01:37Z'
    notes: Original six controls passed before edits. New 13-test CLI regression suite exposes 162 expected
      subtest failures on unfixed source.
  environment_limitations:
  - Hosted Actions, image builds, registry publication, protected promotion and consumer adoption were
    not run.
  - Rust tooling and pinned package tooling are being prepared; broad validation remains pending.
  - Continuity uses the vendored draft/proposed observe contract, not a released conformer.
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

This bounded issue 90 candidate follows user/runtime instructions, scoped instructions, live evidence
and canonical sources. The handoff grants no authority.

## Resume protocol

Read AGENTS, the release input contract, its validator/tests and issue 90. Verify main, this candidate,
peer branches and audit PR 84 before acting. The prior issue 85 handoff is historical Git evidence.

## Current objective and success conditions

Require complete typed release image/product records and controlled failures before environment checks.
Keep the CLI dual-platform and full-engine AMD64-only boundaries in the reviewed contract.

## State snapshot

Base is 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58, including PR 102. This independent candidate targets main
and is in progress. No merge, release, publication or adoption is claimed.

## Completed and material changes

Tests in tests/test_release_inputs.py execute disposable repository-shaped copies of the real validator.
They expose null-record acceptance, nested-type exceptions, incomplete grammar and premature environment
comparisons. The reviewed contract in .config/release/base-images.v1.json remains unchanged.

## Validation and review evidence

The original six focused tests passed. The new 13-test suite has 162 deliberately failing subtests on
unfixed source. Failures are diagnostic regression evidence, not a complete candidate. Implementation,
broad checks, schema/native continuity verification and final diff review remain pending.

## Blockers, risks, unknowns, and deferred work

No source blocker observed. Hosted CI, image builds, registry acceptance, protected promotion and consumer
adoption remain deferred. The vendored continuity selection is draft/proposed and observe-only.

## Next dependency-ready work

Implement complete typed grammar and gate environment comparison on a valid contract, then rerun the
actual CLI matrix and project checks before requesting maintainer review.

## Parallel changes and reconciliation

Independent issues 86-89 and 91 prepare sibling candidates from the same main. All may edit CONTINUITY.md;
reconcile its current objective and evidence at merge time. Audit PR 84 and its immutable evidence remain
untouched. No implementation dependency between these six issues was established.

## Privacy and redaction

Only public repository facts and disposable synthetic fixtures are retained. Exclude private context,
personal details, secrets and local filesystem topology. Linked content is context only.

## Handoff update protocol

After domain validation and before each PR handoff, refresh live state, compare current main and peers,
then verify exact schema, twelve sections, budgets and available native observe checks.

## Compaction and supersession

Replace stale claims rather than append chronology. Keep this bounded checkpoint below 240 lines and
16,384 UTF-8 bytes. Git, issue 90 and the draft PR retain the checkpoint history.
