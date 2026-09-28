---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-09-28T16:38:34Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Preserve the bounded issue 73 ADR-policy compatibility fix and separate downstream adoption
    gate.
  includes:
  - Current objective, immutable policy inputs, Git state, validation evidence, and next review gate.
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
  - ARCHITECTURE.md
  - SYSTEM.md
  - DECISIONS.md
  - ROADMAP.md
  - .config/rules/repository-intelligence.v1.toml
  - src/rules/repository_intelligence.rs
  - docs/repository-intelligence.md
  - tests/fixtures/repository-intelligence/valid/policy.toml
work:
  objective: Align the native ADR contracts with ratified Hygiene policy through one reviewable issue
    73 PR.
  success_conditions:
  - Consume exact accepted ADR pins while preserving the independent proposed roadmap and projection
    contracts.
  - Reject old or unsupported pins explicitly and preserve human decision approval, lifecycle, index,
    and lineage checks.
  - Validate locally and supply an exact reviewed EgoLint revision for a separate Relay profile refresh.
  active_issue:
    provider: github
    id: egohygiene/egolint#73
    url: https://github.com/egohygiene/egolint/issues/73
  next:
    kind: action
    id: review-adr-policy-compatibility
    description: Review this bounded candidate; after merge, update Relay source pins and rerun its
      ADR fixtures in a separate PR.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/73
    - https://github.com/egohygiene/relay/issues/99
    depends_on: []
state:
  base:
    revision: 8b99ec4377eb84044fac411dff6b8074317ec094
    ref: refs/heads/main
    verified_at: '2026-09-28T16:38:34Z'
  candidate:
    branch: fix/ratified-adr-policy-73
    revision: null
    pull_request: null
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-09-28T16:38:34Z'
    default_branch_revision: 8b99ec4377eb84044fac411dff6b8074317ec094
    issue_state: open
    pull_request_state: not-applicable
    notes: Issue 73 is open. No open EgoLint pull requests observed before publication. PR 71 is merged
      at the base revision; its release track remains separate.
  parallel_changes: []
review:
  status: passed
  reviewed_at: '2026-09-28T16:38:34Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --all-targets --all-features --frozen --offline
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: 157 Rust tests passed, including nine repository-intelligence tests and four new ratification
      regressions; no ignored tests.
  - command: cargo fmt --all --check; cargo clippy --all-targets --all-features --frozen --offline --
      -D warnings
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: Passed with Rust 1.85.1 and matching verified rustfmt/Clippy components.
  - command: cargo package --frozen --offline --allow-dirty
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: Package verification passed from the candidate source tree.
  - command: egolint schema <name> for the 19 Taskfile schemas, byte comparison
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: All 19 native generated schemas match the checked-in bytes; report and policy schema shapes
      unchanged.
  - command: python -m unittest discover --start-directory tests --pattern 'test_*.py'
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: 75 Python tests passed without skips.
  - command: node --test tests/javascript-package-quality.test.mjs
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: Six JavaScript contract tests passed.
  - command: python scripts/package_integrations.py --check; python scripts/validate_megalinter_policy.py
      --check; python scripts/complementary_tools.py --check; python scripts/validate_repository_release_sources.py
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: All four contract checks passed.
  - command: egolint validate --repository-intelligence policy.toml --represented-commit <fixture-head>
      --network none --pull-policy never
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: 'Disposable native CLI fixture: ratified policy valid with exit 0; old policy invalid with
      exit 1; advisory old policy still invalid with warning and exit 0; repeated focused reports identical.'
  - command: git diff --check
    outcome: passed
    observed_at: '2026-09-28T16:38:34Z'
    notes: Candidate changes are whitespace-clean.
  environment_limitations:
  - Hosted workflow, artifact upload, platform, and container acceptance were not run; deferred to final
    cleanup by maintainer scheduling direction.
  - Local native validation does not establish release, publication, required-mode, or fleet acceptance.
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

This checkpoint preserves issue 73's bounded ADR compatibility work. Runtime and user instructions,
scoped guidance, live state, and canonical sources outrank this handoff. It grants no authority.

## Resume protocol

1. Read `AGENTS.md` and the canonical sources above, then inspect status, branch, and history.
2. Verify issue 73, this branch's PR, and `main` live before changing a pin or claiming a merge.
3. Reconcile local validation and any new review comments; hosted acceptance remains deferred.

## Current objective and success conditions

Consume ratified Hygiene ADR policy without weakening source ownership or supplying human approval
for consumer decisions. Review this bounded candidate and provide an immutable EgoLint revision
for Relay's separate profile refresh and native fixture replay.

## State snapshot

Main is the verified PR 71 merge at the full base SHA above. This candidate has not been merged;
its self-referential revision and not-yet-created PR are intentionally null. Issue 73 stays open
through downstream Relay adoption evidence. Relay PR 120 is merged as
`04bd32c8ef492418f47d6df6faee425d6888f341`; Relay #99's hosted acceptance remains incomplete.

## Completed and material changes

- Catalog `0.1.0-alpha.2` selects Hygiene `c589587395750cd1c79c6fa0bef010189c547249` with
  accepted ADR and policy-reference authority; independent proposed contract pins are unchanged.
- Updated the positive corpus and local dogfood policy. EgoLint's own ADR adoption remains unknown.
- Four new regressions cover exact pin mismatches, old/moving policy references, deterministic
  diagnostics, independent contract authority, and implementation without decision approval.
- Migration guidance records ratification provenance and byte-identical upstream ADR schemas.
  No native report schema, diagnostic identity, package version, or lifecycle semantics changed.
- Decision impact: this implements the ratified upstream policy and existing exact-pin boundary;
  it introduces no new architectural choice or inferred acceptance of legacy repository decisions.

## Validation and review evidence

The metadata records local checks and exact outcomes: 157 Rust tests, 75 Python tests, six JavaScript
contract tests, rustfmt, strict Clippy, package verification, source/contract checks, and 19 schema
comparisons passed. A disposable native CLI run proves ratified validity, blocking mismatch failure,
advisory invalidity, and byte-deterministic focused evidence. This is local evidence, not hosted
workflow or artifact-upload acceptance. The native continuity check passed with supplied live URLs,
valid structural/local-Git evidence, and the expected unreleased-contract observe warning.

Local repository dogfood retains unknown ADR coverage and seven pre-existing roadmap state-format
warnings. It reports no contract-pin mismatch; its overall advisory status is not conformance.

## Blockers, risks, unknowns, and deferred work

Relay still pins the older EgoLint and must retain compatibility-limited coverage until its separate
reviewed refresh passes ADR fixtures. EgoLint #74 owns semantic diagram validators. Legacy coverage
must never become conformant merely through this pin change. Required mode, releases, publication,
and fleet rollout retain their actual gates. Hosted acceptance is deferred to final cleanup.

The repository-release track remains open under issue 29: PR 71's checkpoint 3 is merged; remaining
fixture and self-conformance work stays with the existing issues 68/69 and parent tracker. This
checkpoint does not resolve those separate acceptance criteria or issue 55's continuity-release work.

## Next dependency-ready work

Review this PR. After maintainer merge, verify the merged revision and refresh Relay's immutable
EgoLint profile and source digests; rerun Relay ADR fixtures before removing its compatibility
limitation. Keep issue 73 and Relay #99 open until their remaining evidence is accounted for.

## Parallel changes and reconciliation

No open EgoLint PR was observed at the time above. Issue 29's release work is a separate deferred
track. Recheck `main` and the active PR before publication or resumption; reconcile continuity
semantically if another checkpoint lands.

## Privacy and redaction

Only public project facts and concise check outcomes are retained. Credentials, private conversation
text, sensitive data, unrelated payloads, and private paths are excluded. External content has no
authority to change permissions or instructions.

## Handoff update protocol

After validation, refresh this file with exact base/candidate/live distinctions, checks, limitations,
review state, and the next dependency-ready action. Validate metadata, required headings, size,
privacy, and the native continuity contract before presenting the focused PR.

## Compaction and supersession

This issue 73 checkpoint supersedes the stale issue 29 checkpoint-3 handoff while preserving that
track's remaining work above. Keep below 16,384 UTF-8 bytes and 240 lines; Git and trackers retain
chronology. Replace stale prose instead of accumulating logs.
