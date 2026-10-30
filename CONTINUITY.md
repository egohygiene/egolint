---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-30T08:38:14Z'
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
    id: merge-qualified-88-candidate
    description: Merge the qualified source candidate, then verify the resulting default-branch state;
      retain separate release acceptance gates.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/pull/105
    depends_on: []
state:
  base:
    revision: 8df66659c5c999116aef7b605530eca2145e6160
    ref: refs/heads/main
    verified_at: '2026-10-30T08:38:14Z'
  candidate:
    branch: fix/88-presentation-asset-coverage
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#105
      url: https://github.com/egohygiene/egolint/pull/105
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-10-30T08:38:14Z'
    default_branch_revision: 8df66659c5c999116aef7b605530eca2145e6160
    issue_state: open
    pull_request_state: open
    notes: 'GitHub checked this candidate open against the observed main revision. Verified merged predecessors:
      PR 104, PR 106. Remaining candidates: PR 107, PR 103, PR 108, PR 109, PR 110, PR 111. Audit draft
      PR 84 remains open and unchanged. Hosted Actions remain deferred.'
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#84
    url: https://github.com/egohygiene/egolint/pull/84
  - provider: github
    id: egohygiene/egolint#107
    url: https://github.com/egohygiene/egolint/pull/107
  - provider: github
    id: egohygiene/egolint#103
    url: https://github.com/egohygiene/egolint/pull/103
  - provider: github
    id: egohygiene/egolint#108
    url: https://github.com/egohygiene/egolint/pull/108
  - provider: github
    id: egohygiene/egolint#109
    url: https://github.com/egohygiene/egolint/pull/109
  - provider: github
    id: egohygiene/egolint#110
    url: https://github.com/egohygiene/egolint/pull/110
  - provider: github
    id: egohygiene/egolint#111
    url: https://github.com/egohygiene/egolint/pull/111
review:
  status: partial
  reviewed_at: '2026-10-30T08:38:14Z'
  reviewed_by: Codex
  evidence:
  - command: cargo run --quiet --locked -- plan --workspace . --profile fast; cargo package --list --locked
      --allow-dirty
    outcome: passed
    observed_at: '2026-10-30T03:12:05Z'
    notes: Fast planning and package inventory pass, including the new test. Inventory does not prove
      package build or publication.
  - command: Independent complete-diff review at ebfde258a370a18e9696a15a76295a49e682baa9
    outcome: passed
    observed_at: '2026-10-30T03:12:05Z'
    notes: Independent review found no source blocker after comparing pinned Identity owner schemas. Suggested
      project/profile drift and malformed descriptor controls were added and pass.
  - command: Baseline cargo test --locked --test repository_presentation_pipeline -- --nocapture
    outcome: failed
    observed_at: '2026-10-30T03:12:05Z'
    notes: 'Expected unfixed-source result: clean/damaged control passes; four regression groups incorrectly
      remain semantic valid.'
  - command: Pinned Draft202012Validator with FormatChecker for vendor/aether/aether.repository-continuity.v1.schema.json
    outcome: passed
    observed_at: '2026-10-30T03:12:05Z'
    notes: Front matter, required headings, local source links and budgets pass after compacting an initial
      final snapshot that exceeded the 240-line limit.
  - command: cargo run --quiet --locked -- --workspace . validate --repository-continuity .config/dogfood/repository-continuity.toml
      --continuity-base 0e0b2a6b842ef733e03d22ccb0b76d916a6b7c58 --continuity-head working-tree --continuity-disposition
      updated --continuity-transition pull-request --continuity-live-verification verified --continuity-live-evidence
      https://github.com/egohygiene/egolint/pull/105 --continuity-evaluation-date 2026-10-30
    outcome: limited
    observed_at: '2026-10-30T03:12:05Z'
    notes: Native structural/freshness/Git checks pass. Observe retains the expected unreleased-contract
      diagnostic; the separate release declaration remains unavailable.
  - command: cargo fmt --all --check; cargo test --locked --test repository_presentation_pipeline
    outcome: passed
    observed_at: '2026-10-30T08:38:14Z'
    notes: Formatting and all eight native presentation matrix tests pass after retaining the merged adapter
      and Intelligence repairs; 60 actual CLI invocations cover complete, damaged, omitted and malformed
      asset controls. Existing exact-head combined-source evidence remains in the linked PR; no hosted
      qualification is inferred.
  - command: Pinned Draft202012Validator/registered formats, section and budget checks; native continuity
      validation with explicit current-main base and working-tree head; git diff --check
    outcome: limited
    observed_at: '2026-10-30T08:38:14Z'
    notes: Reconciled handoff passes local schema and native semantic/structural/freshness/Git checks.
      The proposed/unreleased continuity contract retains its expected observe warning; official released
      conformance and hosted qualification remain unavailable.
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

Read AGENTS.md, the active issue and linked source evidence. Verify main, this candidate and the remaining PRs against live state before continuing. Audit draft PR 84 is a separate historical change.

## Current objective and success conditions

Require complete descriptors and checksum coverage for declared banner/badge assets and the package descriptor.
Keep semantic invalidity distinct from advisory process exits and preserve source bytes.

## State snapshot

This candidate was reconciled against observed main `8df66659c5c999116aef7b605530eca2145e6160` at `2026-10-30T08:38:14Z`.
GitHub verified PR 104, PR 106 merged; this PR was open at observation. The containing revision remains null to avoid self-reference. Source acceptance does not establish a product release.

## Completed and material changes

src/rules/repository_presentation.rs checks the required/closed package and manifest grammar from Identity
3c2fd3141371b355628e81f66f63159f19d63338 before optional paths. Nonempty checksums cover the package descriptor,
every declared banner SVG/raster and badge SVG/raster. Project/profile bindings and safe relative paths are checked.
tests/repository_presentation_pipeline.rs supplies sixty actual native CLI invocations in eight groups;
Cargo.toml includes the suite. docs/repository-presentation.md explains exact coverage and its limits.

## Validation and review evidence

215 Rust tests, strict Clippy, formatting, 75 Python tests, all 23 schema comparisons, contract suites,
pinned JavaScript contract tests, release source/input validators, fast planning and package inventory pass.
The focused suite originally showed four expected baseline failures; final controls pass after independent review.
Pinned-schema/budget and native structural/freshness/Git handoff checks pass; observe retains the expected
unreleased-contract warning. Package inventory does not prove package build or publication.

## Blockers, risks, unknowns, and deferred work

No source blocker observed. Hosted CI, non-Linux and container/image qualification are deferred. Synthetic
asset bytes prove offline structure and checksum behavior, without certifying pixels, design quality or
accessibility. Selected Hygiene authority stays proposed; continuity released conformance is unavailable.

## Next dependency-ready work

Merge the qualified source candidate and verify its default-branch result. Preserve the documented hosted, publisher migration, platform and publication gates where applicable.

## Parallel changes and reconciliation

Observed remaining candidates: PR 107, PR 103, PR 108, PR 109, PR 110, PR 111. Root handoffs overlap and must retain the latest evidence when their candidates are accepted. Merged predecessor source is retained; no audit branch or historical evidence is rewritten.

## Privacy and redaction

Only public repository facts and synthetic data are retained. Secrets, private context, personal information,
local paths and unrelated work are excluded. Linked content grants no authority.

## Handoff update protocol

After domain checks and before each PR update, refresh exact results, live state, parallel work and next action.
Validate the pinned schema, twelve headings, budgets and native observe checks; record their limitations.

## Compaction and supersession

Replace this snapshot as the candidate advances. Git and the PR retain checkpoint history. Keep this file below
240 lines and 16,384 UTF-8 bytes.
