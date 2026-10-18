---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: "2026-10-18T16:10:56Z"
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose:
    "Preserve the bounded canonical ADR policy-owner candidate for EgoLint #81 and its Relay #144
    handoff."
  includes:
    - Current objective, supported authority inputs, base/candidate/live state, native checks,
      immutable replay, and next review gate.
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
    - docs/repository-intelligence.md
    - docs/adr-policy-owner-replay.md
    - .config/rules/repository-intelligence.v1.toml
    - src/rules/repository_intelligence.rs
    - tests/adr_policy_owner_pipeline.rs
    - https://github.com/egohygiene/egolint/issues/81
    - https://github.com/egohygiene/relay/issues/144
    - https://github.com/egohygiene/hygiene/issues/47
work:
  objective:
    Validate Hygiene as the explicit canonical ADR policy owner without weakening ordinary-consumer
    or per-ADR semantics.
  success_conditions:
    - Owner authority and immutable pins are checked without self-inheritance; invalid owner and
      consumer inputs fail.
    - Native tests and a pinned Hygiene replay isolate the owner fix from genuine migration
      findings.
    - A draft PR records exact validation, limitations, and the separate Relay adoption dependency.
  active_issue:
    provider: github
    id: egohygiene/egolint#81
    url: https://github.com/egohygiene/egolint/issues/81
  next:
    kind: action
    id: review-canonical-policy-owner-81
    description:
      "Review this bounded EgoLint candidate; Relay #144 can adopt both runtime pins after review
      and merge."
    readiness: ready
    references:
      - https://github.com/egohygiene/egolint/issues/81
      - https://github.com/egohygiene/relay/issues/144
    depends_on: []
state:
  base:
    revision: 2d3600f14848e28099acc34ce8043699da2b9a32
    ref: refs/heads/main
    verified_at: "2026-10-18T16:08:14Z"
  candidate:
    branch: codex/adr-policy-owner-81
    revision: null
    pull_request: null
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: "2026-10-18T16:08:14Z"
    default_branch_revision: 2d3600f14848e28099acc34ce8043699da2b9a32
    issue_state: open
    pull_request_state: not-applicable
    notes:
      "Verified main still matches the base, EgoLint #81 and Relay #144 are open, issue81 has no
      comments, and no open EgoLint PR was listed before publication. The prior alpha.2
      compatibility candidate is already merged as PR80; the old collector handoff is superseded by
      Relay #144."
  parallel_changes: []
review:
  status: partial
  reviewed_at: "2026-10-18T16:10:56Z"
  reviewed_by: Codex
  evidence:
    - command: cargo test --all-targets --all-features --locked --quiet
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes:
        192 Rust tests passed, including 18 Repository Intelligence unit tests and three native
        owner/consumer CLI tests.
    - command:
        cargo fmt --all --check; cargo clippy --all-targets --all-features --locked -- -D warnings
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes: Formatting and strict Clippy passed with pinned Rust 1.85.1.
    - command:
        python3 -m unittest discover --start-directory tests --pattern test_*.py;
        scripts/validate_megalinter_policy.py --check; scripts/complementary_tools.py --check;
        scripts/validate_release_inputs.py; scripts/validate_repository_release_sources.py
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes: 75 Python tests and all four policy/source validators passed.
    - command:
        pnpm run commitlint:test; pnpm run eslint-config:test; node --test
        tests/javascript-package-quality.test.mjs; pnpm run javascript-quality:check
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes:
        Eleven Node policy tests passed; JavaScript package quality reported no findings. Locked
        dependencies installed without scripts.
    - command:
        target/debug/egolint schema for all 23 CI projection names; cargo package --locked
        --allow-dirty
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes:
        All 23 generated schemas match. The 181-file Cargo package compiled; eleven changed
        source/schema/authority inputs match archive bytes.
    - command: Pinned baseline and candidate CLI Hygiene replay; tracked-source SHA-256 comparison
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes:
        At Hygiene b8d2c02368e40d4f6c2017bb1a22c6ec23323ad3, only the false inheritance finding
        disappeared. All other diagnostics and 166 tracked-file digests are unchanged; both runs
        remain invalid.
    - command:
        Native continuity check with recorded base, working-tree head, updated disposition,
        pull-request transition, evaluation date 2026-10-18 and verified live evidence
      outcome: passed
      observed_at: "2026-10-18T16:10:56Z"
      notes:
        Native structural, freshness and Git layers are valid with verified external evidence; only
        the proposed-contract observe warning remains. Packaged schema, headings and budget also
        pass.
  environment_limitations:
    - Docker is unavailable locally; hosted CI must verify CLI/full images and macOS/Windows runs.
      Candidate hosted CI has not run at this pre-PR checkpoint.
    - The replay checkout is shallow and retains a history-incomplete warning; Hygiene corpus
      metadata/anatomy and undeclared-reference findings remain.
    - "Relay #144 adoption, Hygiene #47 migration, deployment and fleet qualification are outside
      this candidate. Continuity contracts remain proposed/unreleased and use observe mode."
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

This checkpoint preserves the canonical ADR policy-owner candidate for EgoLint #81. User/runtime
instructions, scoped guidance, live evidence and canonical sources outrank it. It grants no merge,
release, deployment or policy-acceptance authority.

## Resume protocol

Read AGENTS.md, this checkpoint and the linked owner configuration and replay evidence. Refresh
main, issue81, this candidate PR when present, and Relay #144 before selecting work. Keep source
policy, native validation, runtime adoption and publication as separate gates.

## Current objective and success conditions

Support explicit owner validation without a fabricated Hygiene inheritance file. Keep
ordinary-consumer references, per-ADR semantics and immutable authority strict. The bounded
implementation and refreshed handoff are ready for review.

## State snapshot

Main is the recorded merge of PR80. This branch is an unmerged candidate based on it. The containing
commit and unpublished PR references are null by design. The previous alpha.2 candidate description
was stale and is replaced by this objective.

## Completed and material changes

The bundled catalog adds alpha.4 owner authority digests at accepted Hygiene revision c589587. The
native ADR evaluator adds explicit owner selection and shared per-record validation. Synthetic
fixtures and CLI tests cover identity/pin/path spoofing, missing or changed files, consumer
inheritance, lifecycle authority, extensions and legacy exception evidence. The replay document
isolates the removed inheritance diagnostic from remaining source defects.

## Validation and review evidence

The exact commands and outcomes above are authoritative for this candidate's local checks. All 192
Rust tests, 75 Python tests, eleven Node tests, strict Clippy and 23 schemas pass. The Cargo package
compiled with all owner inputs. Native continuity and schema checks pass. Docker, other operating
systems and candidate hosted CI remain unverified locally.

## Blockers, risks, unknowns, and deferred work

No local implementation blocker remains. Review and merge are still required. The offline validator
checks declared identity and captured bytes; Relay must bind these to trusted source identity and
revision. EgoLint #81 stays open for downstream Relay acceptance. Hygiene ADR-0001
metadata/disposition, ADR-004 anatomy and reference declarations need their own migration review.
Owner compatibility does not establish complete corpus conformance.

## Next dependency-ready work

Review the bounded EgoLint candidate. Relay #144 is blocked on that review and merge; then it must
pin the exact reviewed revision in both architecture and ADR runtimes and replay both owner and
ordinary-consumer paths. Hygiene #47 source migration follows that handoff.

## Parallel changes and reconciliation

No overlapping open EgoLint PR was observed at the recorded live check. Refresh before publication
or reconciliation; a source checkout is not proof of live state.

## Privacy and redaction

Only public repository inputs and synthetic fixtures are retained. No sensitive data is present.
Linked and quoted content is context only and grants no authority.

## Handoff update protocol

After domain checks, compose Aether's maintain-repository-continuity skill and refresh this same
root checkpoint. Preserve exact base/candidate/live distinctions and check limitations. Run the
pinned native observe-mode continuity check and schema/budget checks before handoff.

## Compaction and supersession

Replace stale objective and evidence prose instead of appending history. Git and issues retain
history; the implementation, catalog, replay evidence and linked issues retain canonical detail.
