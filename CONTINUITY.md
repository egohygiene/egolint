---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-11-14T20:17:11Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Detect ASCII path-prefix collisions including inferred directories without repeating ancestor-only
    descendant noise.
  includes:
  - Portability case topology, stable native findings and Linux Git/CLI regression controls.
  excludes:
  - Release publication, policy authority promotion, fleet rollout and historical audit rewriting.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - docs/portability-rules.md
  - .config/rules/portability.toml
  - https://github.com/egohygiene/egolint/issues/95
  - https://github.com/egohygiene/egolint/issues/42
work:
  objective: Detect ASCII path-prefix collisions including inferred directories without repeating ancestor-only
    descendant noise.
  success_conditions:
  - Detect file/directory, ancestor and ordinary leaf casing conflicts deterministically.
  - Avoid repeating an ancestor-only conflict for each identically named descendant.
  - Keep ASCII-only semantics and unexecuted platform limits explicit.
  active_issue:
    provider: github
    id: egohygiene/egolint#95
    url: https://github.com/egohygiene/egolint/issues/95
  next:
    kind: action
    id: review-ascii-topology
    description: Review this focused M2 topology repair, then continue the independently ready static
      matrix shell diagnostic under issue96.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/95
    - https://github.com/egohygiene/egolint/issues/96
    - https://github.com/egohygiene/egolint/issues/42
    depends_on: []
state:
  base:
    revision: 6658ba6e2f3c6db4410b6e8a9dc9f60695bc743c
    ref: refs/heads/main
    verified_at: '2026-11-14T20:17:11Z'
  candidate:
    branch: fix/portability-prefix-collisions-95
    revision: null
    pull_request: null
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-11-14T20:17:11Z'
    default_branch_revision: 6658ba6e2f3c6db4410b6e8a9dc9f60695bc743c
    issue_state: open
    pull_request_state: not-applicable
    notes: Verified PR112/113/115/116 merged with ordinary parents and exact reviewed trees. This source
      candidate follows accepted main; issue95 remains open before review. No hosted run or publication
      inferred.
  parallel_changes: []
review:
  status: partial
  reviewed_at: '2026-11-14T20:17:11Z'
  reviewed_by: Codex
  evidence:
  - command: cargo test --locked --offline --lib rules::portability::tests; cargo test --locked --offline
      --test portability_topology_pipeline
    outcome: passed
    observed_at: '2026-11-14T20:17:11Z'
    notes: Rust1.85.1:10 portability unit tests and the actual native pipeline pass. The pipeline covers8
      synthetic Git trees, each twice with unchanged index tree IDs and exact stable source findings;
      nine inventory cases include ordering and Unicode-distinct controls. Linux case-sensitive checkout
      only.
  - command: cargo clippy --locked --offline --lib --test portability_topology_pipeline -- -D warnings;
      cargo fmt --all; git diff --check
    outcome: passed
    observed_at: '2026-11-14T20:17:11Z'
    notes: Strict focused Clippy, rustfmt and whitespace pass. No dependency, suppression, release, workflow
      or historical audit/vendor identity changed.
  environment_limitations:
  - Native Git/CLI topology proof is Linux-only; macOS/Windows case-insensitive checkout behavior is not
    executed or claimed.
  - Hosted and broad platform/image qualification remain deferred; release/adoption/human lifecycle gates
    remain separate.
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

Repair the bounded M2 ASCII topology gap. Runtime/user instructions, scoped instructions,
verified live state and canonical sources outrank this handoff.

## Resume protocol

Read AGENTS.md, roadmap 42 and the case-rule guide; verify main and this review branch before acting.

## Current objective and success conditions

Detect inferred ancestor and file/directory collisions, preserve ordinary leaf detection, and
avoid duplicate descendant findings caused solely by an ancestor spelling conflict.

## State snapshot

Front matter separates immutable base, unmerged candidate and time-qualified live state.
PR 112/113/115/116 are merged. Functional source 88bda8376f25e4dac93f7a1df038c3c93f79297b
contains the topology and domain controls; its containing checkpoint revision remains null.

## Completed and material changes

Every path prefix contributes component spellings to an ASCII-folded node. One finding describes
all conflicting spellings at that node and uses a stable existing source path. Identical descendant
components add no repeated ancestor-only finding; consistently cased shared directories remain valid.

## Validation and review evidence

Ten portability unit tests and an actual native pipeline pass. Eight synthetic Git trees run twice,
retaining index tree IDs and stable findings. Nine inventory cases preserve ASCII-only semantics
and reversed-input ordering. Strict focused Clippy, rustfmt and whitespace pass.

## Blockers, risks, unknowns, and deferred work

Native checkout proof is Linux-only; no macOS/Windows checkout compatibility is claimed.
Hosted, broad platform/image, release/adoption and human policy gates remain separate.

## Next dependency-ready work

Review/integrate this repair, then continue the independent static matrix shell diagnostic issue 96.

## Parallel changes and reconciliation

No other open EgoLint source PR was observed after 115/116 integration. Recheck before merging.

## Privacy and redaction

Only public source and synthetic fixtures are retained; private context, credentials and raw logs
stay excluded. Current native fixtures use no real person or external service.

## Handoff update protocol

Refresh after domain checks; verify actual target/live state, pinned schema/formats and size budgets.

## Compaction and supersession

Keep twelve headings, at most 240 lines and 16384 bytes. Canonical owners and Git retain authority/history.
