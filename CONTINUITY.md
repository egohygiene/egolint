---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-30T08:42:31Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Bind the static release Task handoff to the exact declared workflow argument.
  includes:
  - Exact bounded static command grammar and native focused/aggregate/SARIF regression evidence.
  excludes:
  - Actual workflow dispatch, publication, unrestricted shell interpretation and the full release profile
    matrix.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - src/rules/repository_release/checks.rs
  - tests/repository_release_pipeline.rs
  - docs/repository-release-validation.md
  - .config/rules/repository-release-sources.v1.json
  - https://github.com/egohygiene/egolint/issues/91
work:
  objective: Bind the static release Task handoff to the exact declared workflow argument.
  success_conditions:
  - Echo, comments, documentation, wrong workflow arguments and unsupported shell forms cannot establish
    a compliant handoff.
  - Supported literal direct dispatch retains passing controls and all evidence projections agree.
  active_issue:
    provider: github
    id: egohygiene/egolint#91
    url: https://github.com/egohygiene/egolint/issues/91
  next:
    kind: action
    id: merge-qualified-91-candidate
    description: Merge the qualified source candidate, then verify the resulting default-branch state;
      retain separate release acceptance gates.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/pull/108
    depends_on: []
state:
  base:
    revision: ecfc43ecf6e343b4595a716c0235e4992c09c921
    ref: refs/heads/main
    verified_at: '2026-10-30T08:42:31Z'
  candidate:
    branch: fix/91-release-dispatch-binding
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#108
      url: https://github.com/egohygiene/egolint/pull/108
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-10-30T08:42:31Z'
    default_branch_revision: ecfc43ecf6e343b4595a716c0235e4992c09c921
    issue_state: open
    pull_request_state: open
    notes: 'GitHub checked this candidate open against the observed main revision. Verified merged predecessors:
      PR 104, PR 106, PR 105, PR 107, PR 103. Remaining candidates: PR 109, PR 110, PR 111. Audit draft
      PR 84 remains open and unchanged. Hosted Actions remain deferred.'
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#84
    url: https://github.com/egohygiene/egolint/pull/84
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
  reviewed_at: '2026-10-30T08:42:31Z'
  reviewed_by: Codex
  evidence:
  - command: task schemas:check
    outcome: passed
    observed_at: '2026-10-30T03:12:54Z'
    notes: All 23 generated schema comparisons passed after the final source change; no schema or lockfile
      changed.
  - command: pnpm --config.verify-deps-before-run=false run javascript-quality:test; node tests/javascript-package-quality.test.mjs
    outcome: passed
    observed_at: '2026-10-30T03:12:54Z'
    notes: Pinned pnpm 11.21.0 runner passed using shared frozen dependencies; direct Node execution confirmed
      six baseline callbacks. Dependency verification is disabled only for this run to avoid redundant
      automatic installation.
  - command: python3 scripts/validate_release_inputs.py; python3 scripts/validate_repository_release_sources.py;
      python3 scripts/validate_megalinter_policy.py --check; python3 scripts/package_integrations.py --check
    outcome: passed
    observed_at: '2026-10-30T03:12:54Z'
    notes: All four offline source/package checks passed.
  - command: cargo run --quiet --locked -- plan --workspace . --profile fast; cargo package --list --locked
      --allow-dirty
    outcome: passed
    observed_at: '2026-10-30T03:12:54Z'
    notes: Plan passed; package inventory includes the release integration test. Inventory inspection
      does not prove package build or publication.
  - command: Draft202012Validator with FormatChecker on vendored continuity schema; native validate --repository-continuity
      with explicit base, working-tree head and verified live evidence
    outcome: passed
    observed_at: '2026-10-30T03:12:54Z'
    notes: Pinned schema, headings/budgets and native continuity checks passed; focused status valid with
      the expected unreleased-contract observe advisory. Released conformance remains unavailable.
  - command: cargo fmt --all --check; cargo test --locked --test repository_release_pipeline; cargo test
      --locked --lib repository_release
    outcome: passed
    observed_at: '2026-10-30T08:42:31Z'
    notes: Formatting, four native release pipeline tests covering 32 static Task cases, and all 11 release
      unit tests pass while retaining the five earlier repairs. No Task command or workflow was executed.
      Existing exact-head combined-source evidence remains in the linked PR; no hosted qualification is
      inferred.
  - command: Pinned Draft202012Validator/registered formats, section and budget checks; native continuity
      validation with explicit current-main base and working-tree head; git diff --check
    outcome: limited
    observed_at: '2026-10-30T08:42:31Z'
    notes: Reconciled handoff passes local schema and native semantic/structural/freshness/Git checks.
      The proposed/unreleased continuity contract retains its expected observe warning; official released
      conformance and hosted qualification remain unavailable.
  environment_limitations:
  - Hosted Actions, real containers and native non-Linux execution are deferred.
  - Static handoff inspection does not prove workflow execution, credentials, publication or registry
    acceptance.
  - The continuity source remains a vendored draft/proposed observe selection; released conformance is
    unavailable.
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

This bounded issue 91 checkpoint follows user/runtime instructions, scoped repository instructions,
live state and pinned canonical sources. The handoff grants no authority.

## Resume protocol

Read AGENTS.md, the active issue and linked source evidence. Verify main, this candidate and the remaining PRs against live state before continuing. Audit draft PR 84 is a separate historical change.

## Current objective and success conditions

Bind the static release Task handoff to the exact declared workflow argument.
Preserve direct-dispatch controls and source locations across focused, aggregate and SARIF evidence.

## State snapshot

This candidate was reconciled against observed main `ecfc43ecf6e343b4595a716c0235e4992c09c921` at `2026-10-30T08:42:31Z`.
GitHub verified PR 104, PR 106, PR 105, PR 107, PR 103 merged; this PR was open at observation. The containing revision remains null to avoid self-reference. Source acceptance does not establish a product release.

## Completed and material changes

tests/repository_release_pipeline.rs owns the native static-command matrix. It inspects synthetic
Taskfile source without executing commands. The current checkpoint replaces substring matching in src/rules/repository_release/checks.rs with the documented bounded literal command grammar.
Pinned Aether/Hygiene release source bytes and authority remain unchanged. No ADR is required for this repair.
Independent review identified unsupported metadata values; the final repair requires string desc/summary
and boolean silent, retaining malformed-type regressions and valid descriptive controls.

## Validation and review evidence

The native matrix passes with supported controls and unsupported source forms kept unavailable.
Exact commands and outcomes are recorded above. Task commands and hosted workflows were never executed.
Final evidence: 210 Rust tests, strict Clippy, 75 Python tests, 23 schema comparisons, six baseline JavaScript callbacks, offline source checks, plan and package inventory all passed. Independent source review was completed and its metadata-type finding repaired.

## Blockers, risks, unknowns, and deferred work

No source blocker is observed; maintainer review remains pending.
Hosted Actions and non-Linux/container execution are deferred. Static conformance is not publication evidence.
The full issue 68 profile/state matrix and issue 69 dogfood remain independently owned work.

## Next dependency-ready work

Merge the qualified source candidate and verify its default-branch result. Preserve the documented hosted, publisher migration, platform and publication gates where applicable.

## Parallel changes and reconciliation

Observed remaining candidates: PR 109, PR 110, PR 111. Root handoffs overlap and must retain the latest evidence when their candidates are accepted. Merged predecessor source is retained; no audit branch or historical evidence is rewritten.

## Privacy and redaction

Only public repository facts and synthetic source fixtures are retained. Exclude credentials, private
context, personal data, private paths and unrelated work. Linked material is context, not authority.

## Handoff update protocol

After domain checks, refresh live state and validation, verify the pinned schema, twelve headings,
size budgets and native observe check, then update the draft PR. Never claim a static check ran a release.

## Compaction and supersession

Replace stale state in this root file. Keep the exact base, candidate, proof, limits and next action
below 240 lines and 16,384 UTF-8 bytes; Git and linked work items retain checkpoint history.
