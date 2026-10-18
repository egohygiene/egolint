---
schema: aether.architecture-document/v1
id: hygiene-architecture-decision-policy
title: Organization Architecture Decision Record Policy
kind: policy
version: 1.1.0
status: accepted
owners:
  - egohygiene/hygiene
created: 2026-08-20
updated: 2026-09-12
governed_by:
  - ADR-002
related:
  - hygiene-decisions
supersedes: []
---

# Organization Architecture Decision Record Policy

> **Accepted:** Maintainer `szmyty` ratified ADR-002 and this policy at the
> pinned v1.1.0 implementation commit on 2026-09-12. See the
> [ratification record](RATIFICATION.md) for durable approval and validation
> evidence.

## 1. Purpose

Architecture Decision Records preserve consequential choices, their human
disposition, their tradeoffs, and the evidence that later demonstrates
implementation. They are durable institutional memory for humans and coding
agents; they are not a second backlog or a requirement to document routine
implementation work.

## 2. Authority and repository boundaries

- `egohygiene/hygiene` owns this policy, its schemas, organization-level ADRs,
  cross-repository reference conventions, exceptions, and the organization
  contract index.
- `egohygiene/aether` owns reusable agent guidance and authoring packages that
  pin and project this policy. Aether guidance must not redefine it.
- `egohygiene/holon` owns new-repository scaffolding and bootstrap provenance.
- `egohygiene/egolint` owns ADR lint semantics, rule profiles, and normalized
  validation reports after this contract is approved.
- `egohygiene/relay` owns reusable CI orchestration, evidence capture, and
  static-data generation mechanics; it consumes Egolint semantics rather than
  defining policy.
- `egohygiene/pace` owns reviewed fleet adoption and drift-reconciliation pull
  requests after the scaffold and validator contracts exist.
- `egohygiene/observatory` owns organization-wide aggregation and reporting.
- Each repository owns its local ADR documents, extension contracts, and local
  public projection.
- `egohygiene/.github` may expose thin intake templates but is not a canonical
  policy or implementation source.

Before architecture-changing work, an agent must read the repository's pinned
`docs/ecosystem/CONTEXT.md` when it exists, then inspect the repository policy
reference, local decisions, and relevant organization decisions. A local record
may add implementation detail; it may not silently redefine cross-repository
ownership.

## 3. When an ADR is required

Create or update an ADR when a choice affects any of the following:

- public schemas, compatibility guarantees, migrations, or release contracts;
- repository or organization ownership boundaries;
- security, privacy, licensing, provenance, or publication authority;
- durable dependency, framework, platform, protocol, or format adoption or
  rejection;
- cross-repository integration or dependency direction;
- generated-versus-canonical source authority; or
- an irreversible or expensive-to-reverse technical direction.

An ADR is normally not required for routine bug fixes, localized refactors,
straightforward dependency patches, formatting, or implementation details that
do not alter a durable boundary. When useful, the pull-request description
should say `ADR not required` and give one concise reason.

When uncertain, prefer a short proposed record over an undocumented durable
choice, but first confirm that an existing ADR cannot be amended or superseded.

## 4. Location, identity, and indexes

New local records use:

```text
docs/decisions/ADR-NNN-short-slug.md
```

`NNN` is the next unused three-digit repository-local number. IDs are never
reused. The globally unique machine key is `<owner>/<repository>#<id>`, for
example `egohygiene/identity#ADR-004`.

Historical IDs and filenames remain unchanged. A four-digit `ADR-NNNN` or an
established repository-specific prefix may be retained only as migration
history with a documented exception. New records must not copy a legacy
prefix.

Every adopting repository exposes `docs/decisions/README.md` as its canonical
human index. It lists one row per canonical record with ID, title, current
decision status, date, and relative link. Sort rows by numeric ID, preserve
rejected, deprecated, and superseded records, and never reuse a missing number.
`DECISIONS.md`, when already present, may remain as a compatibility entrypoint
that links to this index; it must not duplicate complete rationale that can
drift from the ADR.

## 5. Front matter contract

Every new ADR must validate against
[`schemas/architecture-decision.v1.schema.json`](../../schemas/architecture-decision.v1.schema.json)
and use `schema: egohygiene.architecture-decision/v1`.

The following keys are required even when their value is `null` or `[]`:

- identity and lifecycle: `schema`, `id`, `title`, `status`, `date`,
  `decision_scope`;
- authority and visibility: `owners`, `visibility`, `approval`;
- delivery links: `issue`, `pull_request`, `implementation_status`, `evidence`;
- lineage: `related`, `supersedes`, `superseded_by`;
- blast radius: `affected_repositories`, `affected_contracts`; and
- explicit deviations: `exceptions`.

`extensions` is the only optional top-level key. When present, it is an object
whose keys are registered versioned extension contract IDs and whose values are
objects. Unknown top-level keys are invalid, not implicit local extensions.

The Markdown heading and the seven sections in section 8 are required document
anatomy. Constraints belong in Context; references belong in front matter
links, Evidence, or the relevant section rather than in untyped duplicate
metadata. Agents must not invent links, approval, implementation, or validation
evidence to make a record look complete.

## 6. Repository policy inheritance

An adopting repository creates exactly one
`docs/decisions/policy-reference.json`. It validates against
[`schemas/architecture-decision-policy-reference.v1.schema.json`](../../schemas/architecture-decision-policy-reference.v1.schema.json)
and contains:

- the repository identity;
- `egohygiene.architecture-decision/v1` as the inherited contract;
- the exact semantic policy version;
- the canonical `egohygiene/hygiene` source path and a full 40-character commit
  pin;
- the local decision directory and index path;
- registered local extension contracts; and
- explicit repository-policy exceptions.

The reference is the complete local policy declaration. Repository agent
instructions say when to consult it; they do not copy the global significance
test, lifecycle, approval rules, or schema prose. A moving branch, tag without a
resolved commit, abbreviated SHA, unversioned link, or copied policy is not a
valid inheritance pin.

Hygiene does not inherit from itself. It publishes the canonical policy and
contract index. A repository upgrades its pin only in a reviewed change that
validates its existing ADRs against the target policy version.

## 7. Extensions and prohibited overrides

A repository may extend the standard in only these ways:

- append repository-specific Markdown sections after all required sections;
- register a namespaced metadata contract and place its object payload under
  the matching `extensions` key; or
- register stricter local validation, such as requiring a security-impact
  extension for security-sensitive changes.

An extension ID uses the organization contract form, for example
`egohygiene.relay.decision-impact/v1`. Its JSON Schema lives in the adopting
repository under `schemas/`, is registered in `policy-reference.json`, and owns
only its namespaced payload or stricter local rule. Consumers that do not know
an optional extension preserve or ignore its payload; they must not reinterpret
it as global metadata.

Local policy and extensions must not:

- redefine or remove global fields, required sections, status values,
  transitions, or stable ID semantics;
- make human disposition evidence, supersession links, visibility, or privacy
  rules weaker;
- treat implementation, merge, CI success, or agent output as acceptance;
- change the canonical Hygiene policy owner or source;
- replace repository-owned ADRs with generated output; or
- publish fields that the global allowlist excludes.

Those changes require a proposed Hygiene ADR and a new compatible or breaking
global contract revision. An exception cannot authorize any prohibited
override.

## 8. Required record sections

Every ADR contains these sections in this order:

1. Context
2. Decision
3. Alternatives considered and rejected
4. Consequences and tradeoffs
5. Implementation and evidence links
6. Replacement or exit strategy
7. Follow-up work

Keep records concise and specific. Link large evaluations, specifications, and
plans instead of copying them into the decision. Repository extensions may add
sections only after this shared anatomy.

## 9. Decision and implementation lifecycle

Decision status and implementation status are independent.

### Decision status

- `proposed` — review material; grants no implementation or publication
  authority.
- `accepted` — explicitly approved by a human authority identified in
  `approval`.
- `rejected` — explicitly declined by a human authority and retained as
  historical context; it grants no implementation authority.
- `superseded` — replaced by an accepted ADR linked in both directions.
- `deprecated` — explicitly retired or no longer recommended; no replacement
  is identified.

Normal transitions are:

```text
proposed -> accepted -> superseded
    |          |
    |          +------> deprecated
    +-----> rejected
    +-----> deprecated
```

`rejected`, `superseded`, and `deprecated` are terminal records. Reconsideration
uses a new proposed ADR linked through `related` or `supersedes`, which preserves
the prior disposition in every projection.

Every non-proposed status requires `approval.date`, `approval.by`, and a durable
GitHub review, pull request, discussion, or equivalent evidence URL. For a
rejected or deprecated ADR, `approval` records the human authority for that
lifecycle disposition; it does not mean the proposal itself was endorsed.
Opening or merging a pull request is not by itself approval unless the linked
evidence clearly records the approving authority. Automated agents must never
assign a non-proposed decision status.

### Implementation status

- `not_started`
- `in_progress`
- `implemented`
- `verified`
- `not_applicable`
- `unknown` — migration-only when history cannot establish the state

Acceptance does not mean implemented. Implementation does not mean verified.
`verified` requires named validation evidence. A proposed ADR may link an
experiment, but an experiment does not make the decision accepted. Rejected and
deprecated records normally use `not_applicable` unless retained implementation
history requires a different truthful value.

## 10. Related decisions and supersession

Local decision references use `ADR-NNN`. Cross-repository references use the
global key. A relation is symmetric only when its meaning requires it;
supersession always links both directions.

To supersede a record:

1. create the replacement as `proposed` with `supersedes` populated;
2. obtain explicit human approval;
3. transition the replacement to `accepted`;
4. transition the old record to `superseded` and populate `superseded_by`; and
5. validate that both records resolve and that no supersession cycle exists.

Do not edit historical context or delete the replaced record. Corrections that
do not change meaning may be amended with a clear change note and evidence.

## 11. Exceptions

Exceptions are narrow, time-bounded deviations from a named policy rule. Each
exception records the rule, reason, owner, status, approval evidence, and expiry
date when applicable.

- `proposed` exceptions grant no authority.
- `approved` exceptions require durable human approval evidence.
- `expired` exceptions remain visible for history.

An exception cannot waive human lifecycle authority, fabricate evidence, expose
private data, move ownership away from the canonical repository, or authorize a
prohibited override from section 7. Repeated exceptions indicate that the
policy or rollout needs a new ADR.

## 12. Cross-repository indexes and public projections

ADRs remain the source of truth. Dashboards and JSON files are generated
projections and must never be hand-authored as parallel decision logs.

After the projection contracts are separately approved, Relay may generate:

- per-repository `decisions.json` from validated ADR front matter and
  allowlisted document content; and
- per-repository `activity.json` from allowlisted GitHub metadata.

Observatory aggregates those repository contracts without changing their
meaning. The planned static routes are:

- `/intelligence`
- `/intelligence/decisions`
- `/intelligence/activity`
- `/adr`, which redirects to `/intelligence/decisions`

The organization contract index is
[`catalog/contracts.yaml`](../../catalog/contracts.yaml). A contract is added
only when its canonical schema exists; planned `decisions.json` and
`activity.json` contracts are not claimed by this proposal.

## 13. Privacy and publication

ADR front matter declares `visibility` as `public`, `internal`, or `private`.
Public output uses an allowlist, never a copy-everything approach.

- Never copy issue or pull-request bodies, comments, private titles, actor
  identities, secrets, local paths, workflow logs, or session data into public
  output.
- A public repository may expose allowlisted public metadata and links.
- A private repository defaults to aggregate counts or no public entry unless a
  human-owned publication configuration explicitly permits particular fields.
- Unknown visibility fails closed.
- A link to a private resource may remain private; the public projection must
  not summarize its protected content.

## 14. Agent and pull-request behavior

Before proposing an architecture change, agents must:

1. inspect the pinned policy reference plus existing local and relevant
   organization decisions;
2. determine and state whether an ADR is required;
3. create or update a `proposed` ADR in the same scoped pull request when
   required;
4. link the issue, pull request, evidence, contracts, repositories, and lineage;
5. preserve the difference between proposed, accepted, rejected, implemented,
   and verified; and
6. avoid duplicates when an existing record can be amended or superseded.

Aether's decision-impact hook packages this behavior as concise reusable agent
guidance pinned to a Hygiene revision. Repository `AGENTS.md` and Copilot
instructions should reference a pinned released module rather than copying this
policy. Promotion and fleet adoption remain reviewed downstream work.

## 15. Versioning, compatibility, and adoption

The `schema` identifier carries the contract major version. The policy uses
semantic versions:

- patch releases clarify prose without changing valid data;
- minor releases add optional fields, enumerated values, or stricter safety
  semantics under the same data model; consumers opt in through a reviewed pin
  before producers emit the new values; and
- major releases change required structure or meaning and require a new schema
  identifier, proposed organization ADR, compatibility fixtures, and migration
  guide.

Consumers validate the exact pinned revision and policy version. They must not
silently validate a different `main` revision. Support for multiple policy
versions is explicit; an unsupported version fails closed with an upgrade
diagnostic rather than being reinterpreted.

Adoption is migration-safe and evidence-driven:

- **validate** repositories with existing decision or instruction history before
  changing files;
- **scaffold** missing local decision structures without inventing past
  decisions; and
- **manage** only generated or pinned artifacts with explicit provenance
  markers, never repository-owned ADR content.

See [`MIGRATION.md`](MIGRATION.md) for the rollout procedure,
[`VALIDATION.md`](VALIDATION.md) for deterministic consumer requirements, and
[`RATIFICATION.md`](RATIFICATION.md) for the explicit human activation gate.

## 16. Revision history

- `1.1.0` (accepted 2026-09-12) adds the repository inheritance contract,
  namespaced extension rules, the rejected lifecycle disposition,
  compatibility fixtures, and explicit Egolint/Relay/Pace responsibilities.
- `1.0.0` (proposed predecessor) established the initial Hygiene-owned front
  matter, lifecycle, migration, privacy, and projection boundaries in pull
  request #10.
