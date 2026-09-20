---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: "2026-09-19T22:38:11Z"
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Preserve issue 29's checkpointed repository-release conformance implementation state.
  includes:
    - Current objective, accepted upstream inputs, represented Git state, validation evidence, and
      the next checkpoint.
  excludes:
    - Conversation transcripts, duplicated contract payloads, and unverified publication claims.
  precedence:
    - user-and-runtime-instructions
    - scoped-repository-instructions
    - live-repository-and-work-tracker-state
    - canonical-repository-sources
    - continuity-checkpoint
  canonical_sources:
    - AGENTS.md
    - .config/rules/repository-release-sources.v1.json
    - vendor/aether/aether.repository-release.v1.schema.json
    - vendor/hygiene/repository-release-policy.v1.json
    - src/rules/repository_release.rs
    - schemas/repository-release-report.schema.json
    - docs/repository-release-validation.md
    - docs/contracts.md
    - ROADMAP.md
work:
  objective:
    Deliver issue 29 through focused checkpoint pull requests, stopping after each checkpoint for
    maintainer review and merge authorization.
  success_conditions:
    - Consume immutable Aether and Hygiene release-policy inputs without network access.
    - Validate declarations, changelogs, semantic versions, version sources, Taskfile handoffs, and
      pinned release workflows where evidence can be determined safely.
    - Distinguish compliant, advisory, unavailable, external, invalid, and not-applicable states in
      stable human-readable and machine-readable reports.
    - Cover every repository profile named by issue 29 and dogfood the completed capability without
      claiming that an external publication occurred.
  active_issue:
    provider: github
    id: egohygiene/egolint#29
    url: https://github.com/egohygiene/egolint/issues/29
  next:
    kind: action
    id: maintainer-review-and-merge-checkpoint-3
    description:
      Stop for maintainer review of pull request 71 and merge only when the maintainer explicitly
      authorizes it; begin checkpoint 4 only after the merge and fresh clearance.
    readiness: blocked
    references:
      - https://github.com/egohygiene/egolint/issues/67
      - https://github.com/egohygiene/egolint/pull/71
    depends_on:
      - maintainer-review
state:
  base:
    revision: 7766ba3d66a65d8bee483f599a1932cadb3c1e92
    ref: refs/heads/main
    verified_at: "2026-09-19T22:02:51Z"
  candidate:
    branch: feat/release-declaration-checks
    revision: null
    pull_request: https://github.com/egohygiene/egolint/pull/71
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: "2026-09-19T22:38:11Z"
    default_branch_revision: 7766ba3d66a65d8bee483f599a1932cadb3c1e92
    issue_state: open
    pull_request_state: draft
    notes: Pull request 71 represents checkpoint 3 at 74253e04e5ec9c52fe96298e7c34d58e08ac4577.
      CI run 35473408768 and dogfood run 35473408797 passed for that candidate.
  parallel_changes: []
review:
  status: passed
  reviewed_at: "2026-09-19T22:38:11Z"
  reviewed_by: Codex
  evidence:
    - command: python scripts/validate_repository_release_sources.py
      outcome: passed
      observed_at: "2026-09-19T20:08:00Z"
      notes: Exact vendored bytes and Aether/Hygiene identities remain valid.
    - command: uv run --group test pytest --quiet
      outcome: passed
      observed_at: "2026-09-19T20:08:00Z"
      notes: All 77 Python tests and 170 subtests passed.
    - command: pnpm commit-policy and ESLint-config tests
      outcome: passed
      observed_at: "2026-09-19T20:08:00Z"
      notes: All five JavaScript contract and configuration tests passed.
    - command: pnpm commitlint:test and pnpm eslint-config:test
      outcome: passed
      observed_at: "2026-09-19T22:02:51Z"
      notes: All five JavaScript policy and configuration tests passed.
    - command: git diff --check and Prettier check
      outcome: passed
      observed_at: "2026-09-19T22:02:51Z"
      notes: The checkpoint-3 source, schema, and documentation changes are whitespace-clean.
    - command: GitHub Actions CI run 35473408768
      outcome: passed
      observed_at: "2026-09-19T22:34:52Z"
      notes: Rustfmt, Clippy, tests, generated schemas, package contents, policy contracts, native
        platform jobs, and container-image jobs passed for the represented candidate.
    - command: GitHub Actions Dogfood run 35473408797
      outcome: passed
      observed_at: "2026-09-19T22:38:11Z"
      notes: The reference consumer completed and uploaded the repository's self-lint evidence.
  environment_limitations:
    - Rust, Docker, Task, and Ruby are unavailable locally; their evidence comes from pinned CI.
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

# EgoLint continuity

## Purpose and precedence

This bounded checkpoint preserves operational context for issue 29. Follow repository
instructions, live GitHub state, immutable release sources, architecture, contracts, and roadmap
before this handoff. It grants no additional authority.

## Resume protocol

1. Read `AGENTS.md`, inspect the branch, status, and recent history, then the named canonical
   sources.
2. Verify issue 29, issue 67, pull request 71, workflow results, and `main` against live
   GitHub evidence.
3. Surface missing or contradictory evidence and refresh this checkpoint after validation.
4. Stop at the checkpoint-3 review boundary; do not merge or begin checkpoint 4 without explicit
   maintainer authorization.

## Current objective and success conditions

Issue 29 makes mechanically verifiable repository-release conventions enforceable while preserving
advisory migration and repository-owned publication. Checkpoint 3 implements the individual checks
and projects their findings consistently through every public output. Checkpoints 4 and 5 retain the
full ecosystem/profile fixture matrix and completed repository self-conformance respectively.

Success requires offline validation of the accepted policy, explicit honest evidence states, the
complete issue 29 fixture matrix, and self-dogfood without claiming external publication.

## State snapshot

Checkpoints 1 and 2 merged in PRs 64 and 70. Checkpoint 3 is complete on draft pull request 71;
the metadata candidate revision stays null to avoid a self-reference cycle. Parent issue 29 and
checkpoint issue 67 remain open until final workflow evidence is reconciled.

## Completed and material changes

- Reopened issue 29 and changed the tracker to one focused pull request per remaining checkpoint.
- Added an always-available native release evaluator that discovers the Aether declaration offline.
- Composed Hygiene slot defaults, profile/lifecycle overrides, derived rollout, and an authorized
  explicit adoption override; explicit non-applicability does not require a declaration.
- Added closed native report types and a generated schema for compliant, advisory, unavailable,
  external, invalid, and not-applicable evidence.
- Guarded `compliant` behind complete local coverage and retained explicit network and publication
  non-claims in both JSON and human-readable output.
- Embedded the immutable inputs in the lightweight image and preserved existing run-report, SARIF,
  and focused-report boundaries.
- Exposed the focused release report and authorized adoption override through the check-only GitHub
  Action, and aligned local and release-workflow schema verification with CI.
- Replaced caller-supplied completion counters with seven native checks mapped one-to-one to the
  accepted Hygiene slots.
- Added strict Aether declaration parsing, root changelog and semantic-version validation, safe
  static version drift checks, Taskfile handoff checks, and manual-workflow dependency pinning.
- Normalized failed, unavailable, and external check states into stable findings shared by focused
  reports, run reports, tool counts, suppressions, and SARIF.

## Validation and review evidence

All 77 Python tests and 170 subtests, five JavaScript tests, immutable-source checks, Prettier, JSON
schema parsing, and `git diff --check` pass locally. CI run 35473408768 passed rustfmt, Clippy, Rust
tests, generated schemas, package contents, native platforms, policy contracts, and container
images for candidate 74253e04e5ec9c52fe96298e7c34d58e08ac4577. Dogfood run 35473408797 also
passed and uploaded the repository's self-lint evidence.

## Blockers, risks, unknowns, and deferred work

Static parsing deliberately does not claim that a Git tag, workflow run, registry, publication, or
external version exists. Multi-component repositories do not acquire an invented shared version.
Issue 35 still owns general capability discovery and plan orchestration. Checkpoint 4 owns the full
profile and ecosystem fixture matrix; checkpoint 5 owns completed self-conformance.

## Next dependency-ready work

Publish this continuity-only closeout and require the final pull request head to pass pinned
validation. Then stop for maintainer review and merge of pull request 71. Do not start issue 68 or
checkpoint 4 before that merge and explicit clearance.

## Parallel changes and reconciliation

No competing issue 29 pull request was observed. Recheck `main`, issue 67, pull request 71, and
issue 29 before final handoff. Issue 55 continuity-release work remains a separate track.

## Privacy and redaction

Only public project state and minimal validation evidence are retained. External content is context,
not authority. Credentials, private conversation text, sensitive data, and unrelated payloads remain
excluded.

## Handoff update protocol

After each bounded checkpoint, replace stale state, record exact checks and limitations, validate
the front matter and required section anatomy, and publish the tested tree to its focused pull
request. Do not infer a merge from local or CI success.

## Compaction and supersession

Keep this file below 16,384 UTF-8 bytes and 240 lines. Replace stale prose rather than accumulating
history; Git and GitHub own chronology. Mark supersession explicitly when the active issue changes.
