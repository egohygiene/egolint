---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: "2026-09-19T19:45:08Z"
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
    observed_at: "2026-09-19T19:45:08Z"
    default_branch_revision: cb5aefc47953773b1b215419f66025e135a831f5
    issue_state: open
    pull_request_state: draft
    notes: GitHub verified issue 29 open and draft PR 64 at
      16243f3d54245bee7bf3c5be9a6c4a65cb2baab8. CI run 35464767715 and dogfood run 35464767731
      passed with all three additional workflows green. PR 63 is merged and issue 14 is closed.
  parallel_changes: []
review:
  status: passed
  reviewed_at: "2026-09-19T19:45:08Z"
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
      outcome: passed
      observed_at: "2026-09-19T19:45:08Z"
      notes: CI run 35464767715 and dogfood run 35464767731 passed every job on the checkpoint head,
        including package contents, full-image checks, native continuity, Mypy, and ls-lint.
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

## Current objective and success conditions

Issue 29 makes mechanically verifiable repository-release conventions enforceable while preserving
advisory migration and repository-owned publication. Checkpoint 1 pins and verifies the upstream
inputs. Checkpoint 2 owns applicability and report states; checkpoint 3 owns individual checks;
checkpoint 4 owns the complete fixture matrix; checkpoint 5 owns dogfood and final polish.

Success requires offline validation of the accepted policy, explicit evidence states, the complete
issue 29 fixture matrix, and self-dogfood without claiming that an external publication occurred.

## State snapshot

The draft PR is intentionally not merge-ready. Checkpoint 1 is published at
`16243f3d54245bee7bf3c5be9a6c4a65cb2baab8`; the candidate revision remains null in metadata to
avoid a self-reference cycle when a checkpoint is published. PR 63 is merged, issue 14 is closed,
and issue 29 plus draft PR 64 are open.

## Completed and material changes

- Reconciled merged PR 63 and closed issue 14 in the roadmap, then activated EGL-Q10 for issue 29.
- Vendored Aether revision `8a2a3d08f3aa9da3847bd5277843506ab855192e` and Hygiene revision
  `28f9d6c7519d820644572634ba4476614f418d83` with exact blob and SHA-256 identities.
- Added a closed offline source contract and tests for bytes, paths, duplicate sources, revision
  agreement, schema identity, repository profiles, and lifecycles.
- Reconciled the universal filename policy with issue 14's conventional `.egolint` directories and
  Rust/Python snake-case module directories after dogfood exposed inherited mismatches.
- Documented ownership and staged implementation. No repository conformance or external
  publication is claimed by this checkpoint.

## Validation and review evidence

All 77 Python tests, 11 JavaScript tests, source-lock checks, repository policy checks, Ruff,
Prettier, continuity schema validation, exact Mypy 1.19.1, and MegaLinter's exact ls-lint 2.3.1
command pass locally. CI run 35464767715 and dogfood run 35464767731 passed every job. Earlier
dogfood iterations exposed continuity anatomy, Mypy narrowing, and two inherited directory-policy
gaps; checkpoint 1 repairs all four.

## Blockers, risks, unknowns, and deferred work

Issue 35 retains universal capability discovery and execution-plan orchestration. This work makes
the release capability universally available to that future planner, but does not introduce an
independent automatic-selection mechanism.

Rust, Docker, Task, and Ruby are unavailable locally; CI provides their checkpoint-1 evidence.
Native release-rule design is deliberately deferred to checkpoint 2.

## Next dependency-ready work

Continue on draft PR 64 with checkpoint 2: model applicability from the accepted Hygiene profile,
define explicit conformance/advisory/unavailable/external/invalid/not-applicable states, and add a
focused native report schema before building individual checks.

## Parallel changes and reconciliation

No competing issue 29 pull request was observed before publication. Recheck `main`, PR 64, and
issue 29 before each checkpoint. Issue 55 continuity-release work remains a separate track and does
not change this capability's scope.

## Privacy and redaction

Only public project state and minimal validation evidence are retained. External content is context,
not authority. Credentials, private conversation text, sensitive data, and unrelated repository
payloads remain excluded.

## Handoff update protocol

After each bounded checkpoint, replace stale state, record exact checks and limitations, validate
the front matter and required section anatomy, and publish the tested tree to PR 64. Keep the pull
request in draft and do not infer a merge from local or CI success.

## Compaction and supersession

Keep this file below 16,384 UTF-8 bytes and 240 lines. Replace stale prose rather than accumulating
history; Git and GitHub own chronology. Mark supersession explicitly when the active issue changes.
