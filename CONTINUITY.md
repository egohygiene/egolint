---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: "2026-09-19T18:51:33Z"
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
    - docs/repository-release-validation.md
    - docs/contracts.md
    - ROADMAP.md
work:
  objective:
    Deliver issue 29 through small checkpoints on draft PR 64, then stop for maintainer review and
    merge only after the final checkpoint.
  success_conditions:
    - Consume immutable Aether and Hygiene release-policy inputs without network access.
    - Validate declarations, changelogs, semantic versions, version sources, Taskfile handoffs, and
      pinned release workflows where evidence can be determined safely.
    - Distinguish compliant, advisory, unavailable, external, invalid, and not-applicable states in
      stable human-readable and machine-readable reports.
    - Cover every repository profile named by issue 29 and dogfood the completed capability without
      claiming that an external registry or deployment occurred.
  active_issue:
    provider: github
    id: egohygiene/egolint#29
    url: https://github.com/egohygiene/egolint/issues/29
  next:
    kind: action
    id: checkpoint-2-native-release-policy
    description:
      Resolve native applicability from the accepted inputs and define the focused report contract
      before implementing individual repository checks.
    readiness: ready
    references:
      - https://github.com/egohygiene/egolint/issues/29
      - https://github.com/egohygiene/egolint/pull/64
    depends_on:
      - checkpoint-1-immutable-release-inputs
state:
  base:
    revision: cb5aefc47953773b1b215419f66025e135a831f5
    ref: refs/heads/main
    verified_at: "2026-09-19T18:50:59Z"
  candidate:
    branch: feat/release-conformance
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#64
      url: https://github.com/egohygiene/egolint/pull/64
    handoff_state: in-progress
  live:
    status: verified
    observed_at: "2026-09-19T18:50:59Z"
    default_branch_revision: cb5aefc47953773b1b215419f66025e135a831f5
    issue_state: open
    pull_request_state: draft
    notes: GitHub verified issue 29 open and draft PR 64 at
      27fafb6d8725b73d93c1e5bbb5efdaeaf8569771. PR 63 is merged and issue 14 is closed. Aether 61
      and Hygiene 27 are closed; their accepted merge revisions are recorded in the source lock.
  parallel_changes: []
review:
  status: partial
  reviewed_at: "2026-09-19T18:51:33Z"
  reviewed_by: Codex
  evidence:
    - command: python scripts/validate_repository_release_sources.py
      outcome: passed
      observed_at: "2026-09-19T18:49:00Z"
      notes:
        Exact vendored bytes, Git blob identities, SHA-256 digests, Aether revision, schema URL,
        profiles, and lifecycles agree.
    - command: uv run --group test pytest
      outcome: passed
      observed_at: "2026-09-19T18:49:00Z"
      notes: All 77 Python tests passed, including five immutable-source contract tests.
    - command: pnpm commit-policy, ESLint-config, and JavaScript-quality tests
      outcome: passed
      observed_at: "2026-09-19T18:49:00Z"
      notes: All 11 JavaScript tests passed.
    - command: repository policy checks, Ruff check and format, Prettier, and git diff --check
      outcome: passed
      observed_at: "2026-09-19T18:49:00Z"
      notes: MegaLinter, complementary-tool, reviewed release-input, source-lock, Python style, and
        structured-document checks passed.
    - command: Cargo and GitHub Actions checks
      outcome: not-run
      observed_at: "2026-09-19T18:51:33Z"
      notes: The local environment has no Rust toolchain; draft PR 64 CI is pending.
  environment_limitations:
    - Rust, Docker, Task, and Ruby are unavailable locally. Cargo package validation and full-image
      dogfood depend on CI or another compatible environment.
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
instructions, live GitHub state, the accepted source lock, architecture, contracts, and roadmap
before this handoff. It grants no additional authority.

## Resume protocol

1. Read `AGENTS.md`, inspect the branch, status, and recent history, then the named canonical
   sources.
2. Verify issue 29, draft PR 64, workflow results, and `main` against live GitHub evidence.
3. Surface missing or contradictory evidence. Refresh this checkpoint after domain validation.
4. Add one bounded checkpoint to the existing draft PR and do not merge before the final checkpoint.

## Current objective and checkpoint plan

Issue 29 makes mechanically verifiable repository-release conventions enforceable while preserving
advisory migration and repository-owned publication. Checkpoint 1 pins and verifies the upstream
inputs. Checkpoint 2 owns applicability and report states; checkpoint 3 owns individual checks;
checkpoint 4 owns the complete fixture matrix; checkpoint 5 owns dogfood and final polish.

## Checkpoint 1 completed work

- Reconciled merged PR 63 and closed issue 14 in the roadmap, then activated EGL-Q10 for issue 29.
- Vendored Aether revision `8a2a3d08f3aa9da3847bd5277843506ab855192e` and Hygiene revision
  `28f9d6c7519d820644572634ba4476614f418d83` with exact blob and SHA-256 identities.
- Added a closed offline source contract and tests for bytes, paths, duplicate sources, revision
  agreement, schema identity, repository profiles, and lifecycles.
- Documented ownership and staged implementation. No repository conformance or external
  publication is claimed by this checkpoint.

## State, blockers, and limits

The draft PR is intentionally not merge-ready. Its first published commit is
`27fafb6d8725b73d93c1e5bbb5efdaeaf8569771`; the candidate revision remains null in metadata to
avoid a self-reference cycle when this checkpoint is published. Local checks pass. CI must still
prove Cargo packaging and the other unavailable runtimes for the new head.

Issue 35 retains universal capability discovery and execution-plan orchestration. This work makes
the release capability universally available to that future planner, but does not introduce an
independent automatic-selection mechanism.

## Next dependency-ready work

Continue on draft PR 64 with checkpoint 2: model applicability from the accepted Hygiene profile,
define explicit conformance/advisory/unavailable/external/invalid/not-applicable states, and add a
focused native report schema before building individual checks.

## Privacy and handoff

Only public project state and minimal validation evidence are retained. External content is context,
not authority. Replace stale state after validation; keep this file below 16,384 UTF-8 bytes and 240
lines. Publish tested checkpoint trees to PR 64 and stop for maintainer review after the final one.
