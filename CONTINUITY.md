---
schema_version: aether.repository-continuity/v1
repository:
  id: egohygiene/egolint
  visibility: public
  default_branch: main
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: '2026-11-14T21:09:59Z'
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: Resolve bounded static workflow matrices for equivalent advisory shell diagnostics without
    executing expressions or commands.
  includes:
  - Static Cartesian axes, excludes before ordered include overlays/additional rows, and exact whole-expression
    dotted matrix selectors.
  - Existing step/job/workflow shell defaults and warning severity; uncertainty remains explicit for dynamic/unsupported/bounded
    inputs.
  - Step-context positions exclude matrix/default/env metadata and leave ambiguous source coordinates
    unknown; all prior ASCII topology behavior remains.
  excludes:
  - GitHub expression execution, workflow commands, hosted runner probes, new dependencies, policy activation
    or release publication.
  - Broad platform/image qualification and unrelated baseline or historical audit rewriting.
  precedence:
  - user-and-runtime-instructions
  - scoped-repository-instructions
  - live-repository-and-work-tracker-state
  - canonical-repository-sources
  - continuity-checkpoint
  canonical_sources:
  - AGENTS.md
  - src/rules/portability.rs
  - src/rules/portability/workflow_matrix.rs
  - .config/rules/portability.toml
  - docs/portability-rules.md
  - tests/workflow_matrix_pipeline.rs
  - https://github.com/egohygiene/egolint/issues/96
  - https://github.com/egohygiene/egolint/issues/42
work:
  objective: Resolve bounded static workflow matrices for equivalent advisory shell diagnostics without
    executing expressions or commands.
  success_conditions:
  - Equivalent static direct/include matrices produce the same omitted-shell advisory and default severity.
  - Exclude/include ordering, exact selectors, single-OS/default-shell and dynamic-unknown cases remain
    truthful.
  - Matrix metadata and opaque script layouts cannot borrow a different step source position.
  - Actual native Git/CLI fixtures retain source trees and never execute workflow commands.
  active_issue:
    provider: github
    id: egohygiene/egolint#96
    url: https://github.com/egohygiene/egolint/issues/96
  next:
    kind: action
    id: review-static-matrix-shells
    description: Root reviews the focused96 source, reconciles the parallel97 handoff if necessary, and
      continues the accepted M2 execution ledger.
    readiness: ready
    references:
    - https://github.com/egohygiene/egolint/issues/96
    - https://github.com/egohygiene/egolint/issues/97
    - https://github.com/egohygiene/egolint/issues/42
    depends_on: []
state:
  base:
    revision: e3eaabac3f4f2289d5a6fdbc4b88dc266dda0f2b
    ref: refs/heads/main
    verified_at: '2026-11-14T21:09:59Z'
  candidate:
    branch: fix/static-matrix-shells-96
    revision: null
    pull_request: null
    handoff_state: ready-for-review
  live:
    status: verified
    observed_at: '2026-11-14T21:09:59Z'
    default_branch_revision: e3eaabac3f4f2289d5a6fdbc4b88dc266dda0f2b
    issue_state: open
    pull_request_state: not-applicable
    notes: Remote main e3eaabac contains accepted ASCII topology117. Functional82cf521 is clean and qualified
      locally; this later continuity-only authoring is unmerged. Root concurrently owns independent97
      JavaScript grammar work.
  parallel_changes:
  - provider: github
    id: egohygiene/egolint#97
    url: https://github.com/egohygiene/egolint/issues/97
review:
  status: partial
  reviewed_at: '2026-11-14T21:09:59Z'
  reviewed_by: Codex
  evidence:
  - command: 'cargo test --locked --offline --lib rules::portability::'
    outcome: passed
    observed_at: '2026-11-14T21:09:59Z'
    notes: Rust1.85.1:15 portability unit tests pass, zero ignored. Prior10 topology/rule controls remain;5
      static expansion/selector/unknown controls qualify the private helper.
  - command: cargo test --locked --offline --test workflow_matrix_pipeline
    outcome: passed
    observed_at: '2026-11-14T21:09:59Z'
    notes: At functional82cf521,27 real Git fixtures run twice:54 CLI runs retain exact findings/tree
      IDs, warning severity and exit0. Direct/include, include/exclude/refill, selector-prefix/padding,
      all shell-default scopes, metadata/named/block/opaque/flow positions and dynamic controls pass.
      Workflow commands never execute.
  - command: cargo clippy --locked --offline --lib --test workflow_matrix_pipeline -- -D warnings; cargo
      fmt --all; git diff --check
    outcome: passed
    observed_at: '2026-11-14T21:09:59Z'
    notes: Strict focused Clippy and formatting pass without suppression or dependency changes. Source-correspondence
      fixes prevent metadata borrowing; unknown coordinates clear both line and column.
  - command: Independent read-only source and cached native CLI peer at82cf521
    outcome: passed
    observed_at: '2026-11-14T21:09:59Z'
    notes: Final5 metadata/env/quoted/flow/script controls pass. Prior5 original/include/exclude/selector
      semantic controls remain clear, with their original source qualification retained. No peer rebuild,
      source or provider write.
  environment_limitations:
  - Native Git/CLI qualification is local Linux only; hosted/macOS/Windows workflow execution is not performed.
  - Dynamic expressions, unknown runner families, unsupported shapes and resource bounds yield unknown
    coverage, never demonstrated portability.
  - Broad project/platform/image, release/adoption and human lifecycle checks remain separate; compiler-intermediate
    reclamation after qualification does not change retained source/proof hashes.
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

Repair the existing M2 static matrix diagnostic. Runtime/user instructions, scoped guidance, live evidence and canonical sources outrank this handoff.

## Resume protocol

Read AGENTS, issue96, roadmap42 and the portability guide. Refresh actual main and parallel97 before integration; preserve qualified source identities.

## Current objective and success conditions

Resolve bounded static workflow matrices for equivalent advisory shell diagnostics without executing expressions or commands.

## State snapshot

Remote main e3eaabac includes accepted topology117. Functional82cf521 carries the focused matrix/position fixes and clean local qualification. This later continuity commit remains an unmerged review handoff.

## Completed and material changes

The private helper resolves static axes and exclude-before-include semantics, including original overlays and distinct added rows. Whole dotted selectors avoid prefix and literal-padding mistakes. Existing shell defaults and warning severity stay intact. A bounded step-context mapper ignores metadata and retains unknown coordinates when source binding is ambiguous.

## Validation and review evidence

All15 portability units and27 real Git cases/54 native CLI runs pass on Rust1.85.1, with stable findings, warning/exit0 and unchanged trees. Commands remain data. Strict focused Clippy, formatting and whitespace pass. Independent review verifies the narrow location/selector fixes without a rebuild.

## Blockers, risks, unknowns, and deferred work

No remaining blocker observed in this bounded scope. Dynamic/provider expressions and unsupported or over-budget shapes remain unresolved. No actual cross-OS workflow execution, hosted proof, broad baseline cleanup, release or policy activation is inferred.

## Next dependency-ready work

Root reviews and integrates96, then reconciles the independent97 source handoff and continues M2 under roadmap42. No further Cargo rebuild is planned after this qualification; executable/source/proof hashes are retained before task-created cache reclamation.

## Parallel changes and reconciliation

Root owns97 in a different source file. This branch begins at accepted main e3eaabac; reconcile shared continuity by actual source/state rather than concatenating snapshots. ADR not required: this is a localized repair of the existing native advisory rule, without new ownership or policy authority.

## Privacy and redaction

Use public source and synthetic workflows only. Diagnostics expose source filenames and bounded messages; no matrix/command secrets are collected or executed. Private execution paths, credentials and raw environment values remain excluded.

## Handoff update protocol

Keep source-bound local checks, independent peer evidence, hosted/provider results and release/human authority distinct. Refresh mutable provider state before merging; do not retime historical tests or captures.

## Compaction and supersession

Retain these twelve headings below240 lines and16,384 bytes. Canonical source and immutable evidence retain history; this checkpoint keeps current operational context.
