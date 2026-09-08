---
schema_version: aether.repository-continuity/v1
repository:
  id: "<owner/repository>"
  visibility: "<public|private|internal>"
  default_branch: "<default-branch>"
  continuity_path: CONTINUITY.md
document:
  status: active
  updated_at: "<RFC-3339-time>"
  max_bytes: 16384
  max_lines: 240
  stale_reason: null
  superseded_by: null
scope:
  purpose: "<why this checkpoint is needed>"
  includes:
    - "<current operational state included here>"
  excludes:
    - "conversation transcripts"
    - "duplicated architecture, roadmap, and changelog content"
  precedence:
    - user-and-runtime-instructions
    - scoped-repository-instructions
    - live-repository-and-work-tracker-state
    - canonical-repository-sources
    - continuity-checkpoint
  canonical_sources:
    - AGENTS.md
    - "<relevant architecture, roadmap, decision, or contract path>"
work:
  objective: "<one current objective>"
  success_conditions:
    - "<observable success condition>"
  active_issue:
    provider: "<provider>"
    id: "<stable issue identifier>"
    url: "<stable issue URL>"
  next:
    kind: "<issue|action>"
    id: "<stable next identifier>"
    description: "<next dependency-ready work>"
    readiness: "<ready|blocked|unknown>"
    references:
      - "<stable URL>"
    depends_on: []
state:
  base:
    revision: "<40-hex-revision-or-unborn>"
    ref: "refs/heads/<default-branch>"
    verified_at: "<RFC-3339-time>"
  candidate:
    branch: "<candidate-branch-or-null>"
    revision: null
    pull_request: null
    handoff_state: "<no-active-change|in-progress|ready-for-review|review-reference-recorded|post-merge-reconciliation|abandoned>"
  live:
    status: "<verified|partial|unavailable>"
    observed_at: "<RFC-3339-time>"
    default_branch_revision: null
    issue_state: "<open|closed|unknown|not-applicable>"
    pull_request_state: "<draft|open|merged|closed|unknown|not-applicable>"
    notes: "<what was checked and what must be rechecked>"
  parallel_changes: []
review:
  status: "<passed|partial|failed|not-run>"
  reviewed_at: "<RFC-3339-time-or-null>"
  reviewed_by: "<reviewer-or-null>"
  evidence:
    - command: "<exact command or named external inspection>"
      outcome: "<passed|failed|limited|not-run>"
      observed_at: "<RFC-3339-time>"
      notes: "<result without fabricated success>"
  environment_limitations: []
privacy:
  classification: "<public-repository|private-repository|internal-repository>"
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

# <Repository> continuity

> This template is intentionally invalid until every angle-bracket placeholder
> is replaced with repository-specific evidence. Remove this notice when done.

## Purpose and precedence

Explain the bounded handoff purpose, what it does not replace, and how conflicts
are resolved using the front-matter precedence.

## Resume protocol

1. Read repository instructions and inspect branch, status, recent history, and
   repository shape.
2. Read the applicable canonical sources named above.
3. Read this checkpoint, then verify mutable issue, pull-request, branch, and
   merge claims against available live evidence.
4. Surface missing, stale, or conflicting evidence.
5. Continue only the dependency-ready work named below unless the user changes
   direction.

## Current objective and success conditions

- Objective: <current objective>
- Success: <observable conditions>

## State snapshot

- Verified base: <revision, ref, and verification time>
- Candidate: <branch and proposed change; do not predict merge>
- Live observation: <time-bounded result and unavailable evidence>

## Completed and material changes

- <material change and its canonical owner or source>

## Validation and review evidence

- `<exact command>` — <passed, failed, limited, or not run; include evidence>

## Blockers, risks, unknowns, and deferred work

- Blockers: <none observed or exact blocker>
- Risks: <none observed or exact risk>
- Unknowns: <none observed or exact unknown>
- Deferred: <none or stable work reference>

## Next dependency-ready work

<Exact next issue or action, readiness, dependencies, and stable link.>

## Parallel changes and reconciliation

<Known parallel pull requests or “None observed,” plus reconciliation state.>

## Privacy and redaction

<Repository visibility, applied redactions, and confirmation that prohibited
private or secret material is absent.>

## Handoff update protocol

After project validation and before presenting, opening, or updating a pull
request, reconcile this snapshot, replace stale state, record exact evidence,
compact it, and include it in the same bounded change. Never claim an open or
unverified pull request is merged.

## Compaction and supersession

Keep this file below 16,384 UTF-8 bytes and 240 lines. Replace stale snapshot
prose rather than accumulating history. Git and the work tracker own chronology.
Mark stale or superseded state explicitly with its required reason or pointer.
