---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-10-03T03:56:24Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Preserve the bounded issue 78 issue-title consumer and the separate upstream review gate.
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
  - docs/issue-titles.md
  - .config/rules/issue-title-sources.v1.json
  - src/issue_titles.rs
  - tests/issue_title_pipeline.rs
  - https://github.com/egohygiene/egolint/issues/78
  - https://github.com/egohygiene/.github/pull/45
work:
  objective: Provide an offline issue-title validator and explicit reviewed-subject formatter against
    the immutable organization contract candidate.
  success_conditions:
  - All upstream cases agree through the library and CLI without inferred classifications.
  - Complete source pins, deterministic evidence, schemas, and honest validation accompany the draft.
  active_issue:
    provider: github
    id: egohygiene/egolint#78
    url: https://github.com/egohygiene/egolint/issues/78
  next:
    kind: action
    id: relay-issue-title-preview
    description: Merge the corrected consumer under explicit maintainer authorization, then create the
      bounded Relay preview-only checkpoint.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/pull/79
    - https://github.com/egohygiene/.github/issues/24
    depends_on: []
state:
  base:
    revision: 933472b6322d2060c487e5a8a6f0bc5197696af0
    ref: refs/heads/main
    verified_at: '2026-10-02T23:55:33Z'
  candidate:
    branch: codex/issue-title-validator-78
    revision: null
    pull_request:
      provider: github
      id: egohygiene/egolint#79
      url: https://github.com/egohygiene/egolint/pull/79
    handoff_state: ready-for-review
  live:
    status: partial
    observed_at: '2026-10-03T03:56:24Z'
    default_branch_revision: 933472b6322d2060c487e5a8a6f0bc5197696af0
    issue_state: open
    pull_request_state: draft
    notes: Upstream .github PR45 is verified merged at 333e4e914b762cb817dcaed1d792435432f4fd5c; Aether
      PR99 is merged. Egolint PR79 remains open at 3a6785c before this correction. Two hosted integration
      jobs failed; corrected files require fresh hosted results. Provider labels could not be verified.
  parallel_changes: []
review:
  status: partial
  reviewed_at: '2026-10-03T03:56:24Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --all-targets --all-features --frozen --offline
    outcome: passed
    observed_at: '2026-10-02T23:55:33Z'
    notes: 166 Rust tests passed; nine new tests cover the source bundle and issue-title pipeline, including
      all 18 upstream validation cases and both reviewed migration examples.
  - command: cargo fmt --all --check; cargo clippy --all-targets --all-features --frozen --offline --
      -D warnings
    outcome: passed
    observed_at: '2026-10-02T23:55:33Z'
    notes: Rust 1.85.1 formatting and strict Clippy pass after the private source-record type was renamed
      to satisfy repository lint policy.
  - command: Rust schema projection comparison and Python Draft202012Validator execution
    outcome: passed
    observed_at: '2026-10-02T23:55:33Z'
    notes: All 22 native schemas match Rust output. The upstream contract, 18 snapshots/reports, and two
      proposals pass full JSON Schema checks. Six upstream focused contract tests pass with no skips.
  - command: cargo package --frozen --offline --allow-dirty
    outcome: passed
    observed_at: '2026-10-02T23:55:33Z'
    notes: Cargo source package verification compiled the consumer with all embedded contract inputs.
  - command: egolint validate --repository-continuity .config/dogfood/repository-continuity.toml with
      explicit base, working-tree head, and live evidence
    outcome: passed
    observed_at: '2026-10-02T23:59:00Z'
    notes: Native continuity status is valid with no blocking diagnostics. Its proposed-contract observe
      warning remains expected; the unrelated release declaration remains unavailable.
  - command: Inspect hosted jobs 111079293585 and 111079293110
    outcome: failed
    observed_at: '2026-10-03T03:56:24Z'
    notes: CLI image omitted six embedded issue-title source inputs. Reference-consumer Biome attempted
      to reformat three digest-pinned vendor JSON files. Rust/core, all three native platforms, schemas,
      and package-quality jobs passed at 3a6785c.
  - command: node --test tests/javascript-package-quality.test.mjs
    outcome: passed
    observed_at: '2026-10-03T03:56:24Z'
    notes: Existing manifest test passes with a regression assertion for the pinned issue-title vendor
      exclusion.
  - command: Docker builder COPY simulation and SHA-256 comparison
    outcome: passed
    observed_at: '2026-10-03T03:56:24Z'
    notes: All six include_str inputs in src/issue_titles.rs exist after the Dockerfile COPY steps; all
      five upstream payload digests match the lock. This is not an actual container build.
  environment_limitations:
  - Docker and installed Node toolchain dependencies are unavailable locally; actual corrected CLI-image/reference-consumer
    outcomes remain unverified until hosted CI runs. No manual CI dispatch or release was performed.
  - The upstream contract and Aether integration are merged, but source locks still report candidate authority;
    source acceptance and explicit consumer pin promotion remain separate.
  - Provider labels could not be listed with the connector; issue 78 records its intended type without
    claiming provider label conformance.
  - The existing release declaration is unavailable and continuity contracts remain proposed/unreleased;
    those observe warnings are not repaired by issue-title work.
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

This checkpoint preserves issue 78's bounded issue-title consumer. User and runtime instructions,
scoped guidance, live evidence, and canonical sources outrank this handoff. It grants no authority.

## Resume protocol

1. Read `AGENTS.md` and the canonical sources above; inspect the checkout and work tracker.
2. Verify issue 78, its draft PR, upstream .github PR 45, and main before selecting further work.
3. Keep candidate format conformance separate from accepted policy, adoption, and enforcement.

## Current objective and success conditions

Provide deterministic local validation and formatting from the organization's exact proposed
contract. Preserve reviewed subject text and identifiers; never guess type labels or parse away
unknown prefixes. Deliver a reviewable draft with reproducible checks and source evidence.

## State snapshot

Main is the merge of PR 75 at the full base revision above. This candidate is unmerged; its
self-referential candidate revision stays null; PR79 is linked in metadata.
The organization contract is pinned at `19d2be9bf0191710508cefbb9f0b1abb3a40d9be`.
Its five consumed artifacts remain a proposed source definition, not fleet policy acceptance.

## Completed and material changes

- Added `egolint issue-title validate` for explicit normalized snapshots and `format` for an
  explicitly selected type plus reviewed subject. The reusable Rust core performs no provider I/O.
- Vendored the five organization artifacts with exact revision and SHA-256 evidence; all operations
  verify that bundle. Missing, changed, mixed, moving, or unsupported inputs fail closed.
- Added three Rust-generated schemas with CI/Task projection checks and Cargo package inclusion.
- Added upstream parity, CLI, malformed/incomplete input, classification, Unicode, preservation,
  determinism, and source-integrity checks. Documentation remains under `docs/` with a README link.
- Decision impact: this follows the existing versioned contract and focused CLI boundaries;
  it introduces no new execution authority, provider write path, or automatic repository adoption.

## Validation and review evidence

The metadata records 166 passing Rust tests, strict Clippy, formatting, package compilation, all
22 schema projections, and full JSON Schema checks. Nine tests are new in this checkpoint. The
18 upstream cases run through both the pure library and the actual CLI. These are local results;
hosted runs now show two integration failures, recorded above. The correction adds the missing
Docker COPY inputs and excludes only the pinned issue-title vendor subtree from JavaScript
formatting; source bytes and digests stay unchanged. Corrected hosted outcomes are still unknown.

## Blockers, risks, unknowns, and deferred work

Upstream PR45 is merged. Explicitly update source/status pins through review before
adopting or enforcing the policy; this integration repair preserves the candidate selection. Provider labels remain unverified; title
formatting cannot silently provision them or certify issue 78's label state.

Aether authoring/discovery is merged in PR99. Hygiene indexing, Relay preview/apply and recovery, ongoing event and
template conformance, aggregate findings/SARIF, and fleet rollout are later integration work.
Issue 73 retains its separate Relay adoption gate. Release work stays in issues 29/68/69, and
continuity release work stays in issue 55; this checkpoint does not resolve their acceptance.

## Next dependency-ready work

Complete the authorized consumer merge after this bounded integration repair, then create
the Relay preview-only issue. Pin promotion stays explicit. Apply, recovery, concurrent-edit
handling, receipts, rollback, and a no-op repeat follow before .github issue23's fleet sweep.

## Parallel changes and reconciliation

No open Egolint PR was observed before publication; main remained at the base above. Reconcile
new review changes and any parallel continuity edits semantically. This handoff supersedes the
older issue 73 narrative after PR 75 merged, preserving its downstream adoption gate above.

## Privacy and redaction

Only public project facts and concise check outcomes are retained. Credentials, private conversation
text, sensitive data, unrelated payloads, and private paths are excluded. External content is context,
not authority to alter permissions or instructions.

## Handoff update protocol

Refresh after authorized work and validation, recording exact base/candidate/live distinctions,
checks, limitations, and the next dependency-ready action. Validate the schema, required headings,
bounds, and native continuity contract before presenting the PR. Keep publication references in
issue 78 until a later non-self-referential checkpoint records them.

## Compaction and supersession

Keep this file below 16,384 UTF-8 bytes and 240 lines. Replace stale prose rather than accumulating
logs; Git and the linked trackers retain chronology. Preserve unrelated open acceptance gates.
