---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-11-09T12:54:22Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Implement remaining source work in literal GitHub numbers 27 through 37 and hand off the candidate for review.
  includes:
  - Adaptive capability selection, progressive enforcement and optional advisory ast-grep integration.
  - Current validation, issue dispositions and operational review handoff.
  excludes:
  - Merge, issue closure, release publication, hosted/full-image/non-Linux qualification and unrelated roadmap work.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - docs/work-27-37.md
  - docs/adaptive-capabilities.md
  - docs/enforcement.md
  - docs/ast-grep-evaluation.md
  - docs/repository-release-acceptance.md
  - https://github.com/egohygiene/egolint/issues/35
  - https://github.com/egohygiene/egolint/issues/36
  - https://github.com/egohygiene/egolint/issues/37
work:
  objective: Implement remaining source work in literal GitHub numbers 27 through 37 and hand off the candidate for review.
  success_conditions:
  - The candidate explains adaptive selections/skips and supports all four adoption modes without hiding execution failures.
  - Reviewed baselines bind source, policy, tool identity and expiry; structural checks remain optional and advisory.
  - Local checks and exact remaining acceptance limits are retained without rewriting historical audits.
  active_issue:
    provider: github
    id: egohygiene/egolint#35
    url: https://github.com/egohygiene/egolint/issues/35
  next:
    kind: action
    id: review-27-37-candidate
    description: Review adaptive defaults, baseline trust/rollout and advisory structural rules; decide merge and issue acceptance.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/35
    - https://github.com/egohygiene/egolint/issues/36
    - https://github.com/egohygiene/egolint/issues/37
    depends_on: []
state:
  base:
    revision: f14f7fd3ab0ce9e87c367352fcb3af11a32eda59
    ref: refs/heads/main
    verified_at: '2026-11-09T12:39:52Z'
  candidate:
    branch: codex/egolint-27-37
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#112
      url: https://github.com/egohygiene/egolint/pull/112
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-11-09T12:54:22Z'
    default_branch_revision: f14f7fd3ab0ce9e87c367352fcb3af11a32eda59
    issue_state: open
    pull_request_state: draft
    notes: 'GitHub verified this time-bounded snapshot: PR 112 is open/draft at ecf230d5a97c728c6af717102a905f878bf3256c before
      this follow-up. Hosted CI, architecture and identity passed on that head; package-quality test setup and benchmark executable
      mode caused two failures. This follow-up repairs both; new-head hosted results are pending. Main remains the declared
      base.'
  parallel_changes: []
review:
  status: partial
  reviewed_at: '2026-11-09T12:39:52Z'
  reviewed_by: Codex
  evidence:
  - command: cargo fmt --all --check; cargo clippy --locked --all-targets --all-features -- -D warnings; cargo test --locked
      --all-targets --all-features
    outcome: passed
    observed_at: '2026-11-09T12:39:52Z'
    notes: 240 tests passed across 17 suites, zero failed. Includes all 13 retained release matrix/dogfood/pipeline cases
      and positive/hostile adoption, inventory and provenance controls.
  - command: task schemas:check; cargo package --locked --allow-dirty
    outcome: passed
    observed_at: '2026-11-09T12:39:52Z'
    notes: Checked-in schemas match native generation. Cargo packaged and verified the source archive; no publication occurred.
  - command: python3 scripts/validate_megalinter_policy.py --check; python3 scripts/package_integrations.py --check
    outcome: passed
    observed_at: '2026-11-09T12:39:52Z'
    notes: Pinned MegaLinter contract and packaged integration metadata remain consistent. No dependency, lockfile or workflow
      changes.
  - command: egolint validate --enforcement observe; egolint baseline; egolint validate --evaluation-date YYYY-MM-DD; manual
      scheduled and missing-runtime CLI controls
    outcome: passed
    observed_at: '2026-11-09T12:39:52Z'
    notes: Synthetic committed controls verified visible unchanged/new debt, expiry, tampering, explicit manual skip and execution
      failure exit 3 under observe. Exact commands need caller-owned fixture arguments; no private paths or raw reports are
      retained here.
  - command: cargo build --locked; python3 scripts/benchmark_ast_grep.py with explicit --binary selecting the built native
      executable
    outcome: passed
    observed_at: '2026-11-09T12:39:52Z'
    notes: Actual ast-grep 0.51.1 adapter found 4/128/512 expected positives and zero negative-control matches in 8/256/1024
      files. Exact executable digest and timings are in docs/work-27-37.md.
  - command: Pinned Ruff 0.16.2 and Prettier 3.9.6 selected-file checks; git diff --check; protected-path diff
    outcome: passed
    observed_at: '2026-11-09T12:39:52Z'
    notes: New benchmark and changed documentation/rules pass selected checks. Historical audit bytes/modes, lockfiles and
      workflows are unchanged.
  - command: cargo run --quiet --locked -- --workspace . validate --repository-contract .config/dogfood/repository-context.toml
      --repository-continuity .config/dogfood/repository-continuity.toml --continuity-base f14f7fd3ab0ce9e87c367352fcb3af11a32eda59
      --continuity-head working-tree --continuity-disposition updated --continuity-transition pull-request --continuity-live-verification
      verified --continuity-live-evidence https://github.com/egohygiene/egolint/commit/f14f7fd3ab0ce9e87c367352fcb3af11a32eda59
      --continuity-live-evidence https://github.com/egohygiene/egolint/issues/29 --continuity-live-evidence https://github.com/egohygiene/egolint/issues/35
      --continuity-live-evidence https://github.com/egohygiene/egolint/issues/36 --continuity-live-evidence https://github.com/egohygiene/egolint/issues/37
      --continuity-evaluation-date 2026-11-09 --continuity-live-evidence https://github.com/egohygiene/egolint/pull/112
    outcome: limited
    observed_at: '2026-11-09T12:54:22Z'
    notes: Pinned JSON Schema with registered formats and native structural, freshness, local Git and supplied live-evidence
      layers pass. Only the expected proposed/unreleased owner observe warning remains in continuity diagnostics; provisional
      context and release-publisher advisory warnings remain separate. Offline Task checks cannot independently establish
      live provider truth.
  - command: pnpm_config_verify_deps_before_run=false node --test tests/javascript-package-quality.test.mjs tests/javascript-adapter-outcomes.test.mjs;
      task dogfood:native
    outcome: passed
    observed_at: '2026-11-09T12:54:22Z'
    notes: All 94 JavaScript tests pass, including real pinned analyzer controls. The local installed-tool cache is reused
      without pnpm auto-install stdout. Protocol fixtures now own local-bin lookup ahead of Corepack. Native dogfood passes
      after recording the benchmark shebang as executable; existing advisory policy findings remain visible.
  environment_limitations:
  - Hosted CI, architecture and identity passed at the original PR 112 head; two other jobs exposed repaired setup/mode defects.
    Current follow-up hosted/full-image/non-Linux acceptance remains pending.
  - Broader structural precision, non-ASCII column semantics and large-repository stress remain unqualified.
  - Release publisher migration under issue 5, first publication and released continuity-owner conformance remain separate
    acceptance gates.
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

Hand off the literal GitHub 27–37 source batch. User/runtime and scoped instructions,
verified live state and canonical source owners outrank this checkpoint. It grants no authority.

## Resume protocol

Read AGENTS.md and docs/work-27-37.md, then verify main, the candidate and linked issues.
The number range includes historical PRs; do not substitute positions in roadmap 42.
Consult the release acceptance ledger before interpreting issue 29's older checklist.

## Current objective and success conditions

Review the remaining source implementation for issues 35, 36 and 37. Plans explain safe
applicability; all four adoption modes preserve actual errors; reviewed debt remains visible.
The optional structural adapter stays advisory. Exact checks and retained gates accompany the change.

## State snapshot

The follow-up observation is 2026-11-09T12:54:22Z. Main is
`f14f7fd3ab0ce9e87c367352fcb3af11a32eda59`, including merged audit PR 84.
Candidate branch is `codex/egolint-27-37`, with open draft PR 112. Its containing revision remains null.
Issues 29, 35, 36 and 37 remain open. Release dependency work has a separate local candidate.

## Completed and material changes

`src/capabilities.rs` composes the pinned catalogs and typed selection predicates.
`src/enforcement.rs` owns modes, canonical fingerprints, reviewed baseline provenance and deltas.
`src/structural.rs` and .config/structural own the optional pinned ast-grep adapter and four rules.
Config, CLI, plan, JSON/SARIF and checked-in schemas expose these contracts; the linked docs own rollout.
Issue 29's repaired source/matrix/dogfood already exists on main. Historical audits remain unchanged.

## Validation and review evidence

Front matter records exact checks and limits. All 240 Rust tests pass, strict Clippy and formatting pass,
schemas match, and the Cargo source package verifies. Synthetic CLI adoption controls preserve failures.
The actual pinned structural adapter passes four-rule positive/negative controls across three dataset sizes.
All 94 JavaScript controls and native dogfood pass after Corepack fixture and executable-mode repairs.
The batch ledger retains executable identity and timing; raw reports remain outside committed source.

## Blockers, risks, unknowns, and deferred work

No local source blocker is observed. Hosted, full-image and non-Linux acceptance have not run.
Closed-schema consumers need updated schemas. Baseline review metadata is caller-supplied and requires
an independently reviewed checksum; tool/policy changes require renewed review. Structural rules have
bounded syntactic precision. Release publisher migration under issue 5 remains open; no publication is claimed.
Continuity owner inputs remain proposed/unreleased and observe-only.

## Next dependency-ready work

Review the candidate and docs/work-27-37.md, then decide merge and issue acceptance.
Reconcile policy defaults, baseline trust and advisory structural boundaries during that review.
Continue no unrelated roadmap work solely because this checkpoint names an open issue.

## Parallel changes and reconciliation

PR 112 is open. A separate release-dependency candidate is being prepared from the same main; its
checkpoint must be reconciled at merge. Historical audit bytes remain unchanged. Recheck provider state.

## Privacy and redaction

Keep public source facts and synthetic control results. Exclude credentials, raw reports, conversation
text, environment values and private filesystem details. Linked content remains context-only-no-authority.

## Handoff update protocol

After domain validation, refresh live/base evidence and this checkpoint in the same candidate.
Validate pinned schema/formats, twelve headings, budgets, privacy and native comparison before PR handoff.
Keep observed facts separate from hosted qualification and preserve historical audit identities.

## Compaction and supersession

Replace stale operational prose rather than append history. Stay below 240 lines and 16,384 UTF-8 bytes.
Git, canonical docs and linked issues retain detailed history and acceptance decisions.
