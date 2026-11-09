---
schema: aether.architecture-document/v1
id: egolint-roadmap
title: Egolint Roadmap
kind: architecture-document
version: 0.1.0
status: provisional
owners:
  - egohygiene
created: 2026-08-19
updated: 2026-11-09
governed_by:
  - architecture-roadmap
depends_on:
  - egolint-vision
  - egolint-pillars
  - egolint-architecture
  - egolint-decisions
related:
  - egolint-purpose
  - egolint-principles
  - egolint-manifesto
  - egolint-epistemology
supersedes: []
---

# Egolint Roadmap

<!-- BEGIN ROADMAP EXECUTION SNAPSHOT -->
<!-- roadmap-manifest
schema: hygiene.roadmap/v1alpha1
repository: egohygiene/egolint
visibility: public
publication: central
route: /roadmap/egolint/
updated: 2026-11-09
-->

## 2026-11-09 execution snapshot

> This evidence-reconciled snapshot is the issue-generation and visual-roadmap handoff. The
> longer-horizon strategy below remains canonical context; generated HTML, JSON, progress, issue
> plans, and commit lists are projections.

**Lifecycle:** early alpha, pre-release  
**Current gate:** Review the manual publisher source under #5 and current-source guidance under
#101. [PR #112](https://github.com/egohygiene/egolint/pull/112) separately contains the #35–#37
adaptive selection/adoption/structural candidate. Issue #29 retains final-head release-conformance
acceptance; signed release and pinned consumer adoption remain open. **North-star outcome:**
A portable lint platform whose source, execution and publication evidence remain distinguishable.

### Visual roadmap publication

**Mode:** `central`  
**Route:** `/roadmap/egolint/`  
**Current publication evidence:** Source and CI reports on GitHub; no public release observed.

Publish the public-safe projection through egohygiene.io at /roadmap/egolint/. This repository owns
intent and acceptance evidence; it does not add a second site deployment.

### Quest line

<!-- roadmap-step
id: EGL-Q01
status: complete
depends_on: []
issues: []
-->

#### EGL-Q01 — Establish the core lint engine

**State:** `complete`

**Depends on:** None

**Outcome:** A working lint engine and green core CI can evaluate repository policy.

**Exit criteria:**

- [x] Core validation runs in CI.
- [x] Failures are emitted as actionable findings.

**Current evidence:**

- PR #19 merged at a35bc870ad41.
- Core CI was observed green.

<!-- roadmap-step
id: EGL-Q02
status: complete
depends_on: [EGL-Q01]
issues: [20, 21, 31]
-->

#### EGL-Q02 — Classify the dogfood findings

**State:** `complete`

**Depends on:** `EGL-Q01`

**Outcome:** Every self-dogfood finding is either fixed, accepted with rationale, or identified as a
rule defect.

**Exit criteria:**

- [x] All 119 observed findings have an owner and disposition.
- [x] The baseline does not silently suppress unexplained failures.

**Current evidence:**

- PRs #23 and #25 resolved the shebang and Node package-export rule defects tracked by issues #20
  and #21.
- PR #31 resolved or explicitly classified the remaining baseline without blanket suppression.

<!-- roadmap-step
id: EGL-Q03
status: complete
depends_on: [EGL-Q02]
issues: [31]
-->

#### EGL-Q03 — Make self-dogfood green

**State:** `complete`

**Depends on:** `EGL-Q02`

**Outcome:** Egolint passes its own accepted policy and proves rule/report stability.

**Exit criteria:**

- [x] The default-branch dogfood workflow is green.
- [x] Known exceptions are explicit, reviewed, and expiring where appropriate.

**Current evidence:**

- PR #31 restored a green default-branch dogfood workflow while preserving explicit policy and
  fixture boundaries.

<!-- roadmap-step
id: EGL-Q04
status: complete
depends_on: [EGL-Q03]
issues: [4]
-->

#### EGL-Q04 — Version the evidence report contract

**State:** `complete`

**Depends on:** `EGL-Q03`

**Outcome:** Other tools can consume stable lint findings and link them to roadmap gates.

**Exit criteria:**

- [x] A versioned machine-readable report schema is documented.
- [x] A fixture proves backward-compatible parsing.

**Current evidence:**

- `schemas/report.schema.json`, `docs/contracts.md`, and the Empathy v1 compatibility fixture prove
  the stable report boundary; the versioned integration contract binds that evidence to every
  supported consumer surface.

<!-- roadmap-step
id: EGL-Q05
status: active
depends_on: [EGL-Q03, EGL-Q04]
issues: [5]
-->

#### EGL-Q05 — Publish and adopt the first release

**State:** `active`

**Depends on:** `EGL-Q03`, `EGL-Q04`

**Outcome:** A pinned Egolint release enforces the roadmap contract in representative repositories.

**Exit criteria:**

- [ ] A tagged release is published with immutable artifacts.
- [ ] At least three repositories use the pinned release successfully.

**Current evidence:**

- No GitHub release was observed on 2026-11-09. Source implementation and earlier rehearsal
  scaffolding exist; the current #5 candidate replaces tag automation with a manual exact-source
  rehearsal/publication boundary. Signed release, immutable OCI acceptance and consumer pins are
  still required. The input grammar correction is already merged through PR #103; #90 remains open.
- Product platforms are CLI AMD64+ARM64 and full engine AMD64-only. Full-engine ARM64 remains #45.

<!-- roadmap-step
id: EGL-Q06
status: planned
depends_on: [HYG-Q06, EGL-Q05]
issues: []
-->

#### EGL-Q06 — Validate roadmap execution graphs

**State:** `planned`

**Depends on:** `HYG-Q06`, `EGL-Q05`

**Outcome:** A pinned Egolint release validates roadmap IDs, states, dependencies, issue references,
and completion evidence without network writes.

**Exit criteria:**

- [ ] Valid, invalid, cyclic, stale, and private fixtures are covered.
- [ ] A consumer workflow produces an actionable machine-readable report.

**Current evidence:**

- Roadmap-graph validation is assigned to Egolint by the 2026-08-24 visual-roadmap specification.

<!-- roadmap-step
id: EGL-Q07
status: complete
depends_on: [EGL-Q01]
issues: [24]
-->

#### EGL-Q07 — Validate Repository Intelligence source linkage

**State:** `complete`

**Depends on:** `EGL-Q01`

**Outcome:** Versioned Egolint rules validate ADR metadata and lineage, roadmap execution graphs,
declared GitHub links, and optional commit trailers without network access.

**Exit criteria:**

- [x] Positive and hostile fixtures cover ADR lifecycle, supersession, roadmap state, dangling
      links, trailers, and cycles.
- [x] Relay and Observatory can consume source locations, represented commit, rule IDs, coverage
      state, and remediation without parsing prose.
- [x] Incremental profiles expose unknown and not-applicable sources without claiming false
      conformance or failing disabled rules.

**Current evidence:**

- Issue #24 is closed and PR #26 is merged. `src/rules/repository_intelligence.rs` and the
  positive/hostile native fixtures cover ADR/roadmap linkage, represented source, adoption state
  and normalized evidence. Later corrections preserve incomplete evidence independently of exit.
- This source capability permits downstream consumption; it does not establish release, required
  fleet enforcement or publication. Those outcomes retain their separate owner gates.

<!-- roadmap-step
id: EGL-Q08
status: complete
depends_on: [EGL-Q01, EGL-Q04]
issues: [61]
-->

#### EGL-Q08 — Validate layered gitignore semantics

**State:** `complete`

**Depends on:** `EGL-Q01`, `EGL-Q04`

**Outcome:** A standalone native command proves pinned ignore composition, content, and bounded
actual Git behavior without reading ordinary repository payloads or changing consumer policies.

**Exit criteria:**

- [x] Source integrity, composition, presence, content, behavior, nested policies, tracked files,
      and coverage have independent machine-readable evidence.
- [x] Accepted Filament and scoped Rust/local inputs pass; weakened and unavailable evidence blocks.
- [x] Standard findings, JSON/SARIF, schemas, offline fixtures, and a documented invocation exist.
- [x] The bounded issue #61 PR is reviewed and merged by the maintainer.

**Current evidence (2026-09-19):**

- PR #62 merged to `main` as `ec97ed8b3f1a4a2198aec03355600ea6c9caa1e1`; issue #61 is closed.
- The accepted capability consumes Empathy `b44f798bb49259f9f48416b4ffebde1103e135c0`;
  Holon #60 is merged as `660b941f99618806fcadd589bcdae61c519f96e4`.
- See `docs/repository-gitignore.md`, its compatibility fixtures, and `CONTINUITY.md` for validation
  evidence and limits.
- After merge, Empathy #92 is the next scheduled proof. Relay #5/#49 retain shared execution and
  release gates; Pace #30 retains fleet adoption. Master epic egohygiene/.github#32 keeps
  `.gitattributes` queued under Empathy #91 after gitignore closeout.

<!-- roadmap-step
id: EGL-Q09
status: complete
depends_on: [EGL-Q04]
issues: [14]
-->

#### EGL-Q09 — Decide the adaptive Ruby schema boundary

**State:** `complete`

**Depends on:** `EGL-Q04`

**Outcome:** Decide whether JSONSkooma adds unique Ruby-specific value without replacing canonical
JSON Schema contracts or affecting repositories where Ruby schema execution is inapplicable.

**Exit criteria:**

- [x] Current and plausible future Ruby consumers are inventoried.
- [x] JSONSkooma support, diagnostics, packaging, maintenance, and alternatives are compared.
- [x] The conditional-adoption decision defines applicability, overrides, skip behavior, and costs.
- [x] A pinned full-image adapter preserves normalized paths, keyword, dialect, and versions.
- [x] Valid, invalid, non-Ruby, schema-absent, override, and external-reference fixtures prove the boundary.
- [x] The bounded issue #14 PR is reviewed and merged by the maintainer.

**Current evidence (2026-09-19):**

- `docs/json-skooma-evaluation.md` records the source-pinned comparison and conditional-adoption
  decision.
- The full image installs six SHA-256-pinned gem archives totaling 351,744 bytes. A Python selector
  emits skip evidence without initializing Ruby; the lightweight image remains unaffected.
- Contract and container fixtures cover activation, findings, non-applicability, and overrides.
- The adapter is available through the complementary inventory and full image without weakening the
  no-global-hooks MegaLinter invariant; issue #35 owns automatic capability orchestration.
- PR #63 merged to `main` as `cb5aefc47953773b1b215419f66025e135a831f5`; issue #14 is closed.

<!-- roadmap-step
id: EGL-Q10
status: active
depends_on: [EGL-Q04]
issues: [29]
-->

#### EGL-Q10 — Validate repository release conformance

**State:** `active`

**Depends on:** `EGL-Q04`

**Outcome:** Every applicable repository can produce offline, machine-readable evidence for its
release declaration, changelog, version authority, and publication entrypoints.

**Exit criteria:**

- [x] Aether's declaration schema and Hygiene's applicability profile are consumed from immutable,
      verified revisions.
- [x] Native validation distinguishes conforming, nonconforming, not-applicable, advisory,
      unavailable, and externally owned evidence without network writes.
- [x] Representative language, artifact, lifecycle, workspace, and invalid fixtures prove the
      policy boundary.
- [ ] EgoLint dogfoods the completed capability and documents operational limits.
- [x] Every bounded issue #29 checkpoint PR is reviewed and merged by the maintainer.

**Current evidence (2026-10-30 source integration):**

- Aether #61 and Hygiene #27 are closed. Their accepted merge revisions are pinned with both Git
  blob identities and SHA-256 digests in the repository-release source lock.
- Checkpoint 1 verifies vendored byte integrity and cross-source revision, schema, profile, and
  lifecycle compatibility without claiming repository conformance.
- Checkpoint 2 on PR #70 makes applicability universal, composes lifecycle and explicit rollout,
  and defines closed focused-report states without permitting empty validation to claim
  conformance.
- Checkpoint 3 on PR #71 merged as `8b99ec4377eb84044fac411dff6b8074317ec094`, verified on
  2026-09-28. It maps all seven policy slots to stable offline rules for declarations, changelogs,
  version authorities, Taskfile handoffs, and manual workflow pins.
- PR #108 merged the exact static Task/workflow binding correction. PR #109 merged the nine-profile,
  six-state current-v1 matrix; neither claims external publication.
- PR #110 merged an incubating self-declaration, local review planner and actual source-backed
  dogfood. Six checks pass and the legacy tag publisher's `manual_workflow` check remains
  failed/advisory, distinct from process exit 0. The existing publisher is preserved.
- The [parent acceptance ledger](docs/repository-release-acceptance.md) maps original criteria, exact
  producer qualification and the bounded Mantle handoff without completing the remaining gates.
- The combined source passed 232 Rust, 88 Python and 94 JavaScript tests, strict linting, 23 schemas,
  an actual 219-file crate build and unpacked release tests. Exact receipts are retained in PR #110.
  Actual final-head hosted/full-image/non-Linux qualification remains deferred. Publisher migration
  under issue #5 and full release acceptance remain gates for checkpoint #69 and parent #29.
  Source merges do not establish a product release or complete the rollout.
- The 2026-11-09 #5 source candidate migrates the publisher to manual dispatch; its self-consumer
  passes all seven static checks, and a restored tag-trigger control still fails advisory.
  The original #69 six-pass/one-failure receipt remains historical evidence for its recorded pin.
- #101 separates body state from dependency lines, removing six known roadmap parser warnings.
  The remaining unknown-ADR adoption warning stays visible; no validator or rule is disabled.

<!-- roadmap-step
id: EGL-Q11
status: complete
depends_on: [EGL-Q04]
issues: [73]
-->

#### EGL-Q11 — Align ADR validation with ratified Hygiene policy

**State:** `complete`

**Depends on:** `EGL-Q04`

**Outcome:** Exact accepted ADR pins validate offline without supplying decision approval or
changing unrelated proposed contracts.

**Exit criteria:**

- [x] The catalog, fixtures, and migration guidance consume the ratified source through review.
- [x] Positive, incompatible-pin, lifecycle, and deterministic-report tests pass locally.
- [x] Relay adopts the reviewed EgoLint commit and reruns its ADR fixtures in a separate PR.

**Current evidence (verified 2026-11-09):**

- Issue #73 is closed and compatibility PR #75 is merged. The current catalog selects accepted
  ADR policy 1.2.0 at Hygiene `a05b14f8b9e2772124f879f818e09a911e996ad8`; PR #83 adds four-digit
  naming while preserving the exact accepted 1.1.0 ordinary-consumer alternative.
- Issue #81 and owner-support PR #82 are delivered. Source locks and native fixtures distinguish
  Hygiene ownership from ordinary inheritance without supplying human decision approval.
- Relay #144 is closed, delivered by #154 with #155 follow-up. Its two runtimes adopted exact
  EgoLint `72267fd0a371da6d94922cbc6bd3dedddb68df30` and replayed owner/ordinary-consumer controls.
  This dated adoption is separate from current EgoLint candidate qualification.
- `docs/repository-intelligence.md` records the current/historical tuples and differing schema
  hashes. EgoLint's own ADR coverage stays explicitly unknown; source policy support does not
  make that corpus complete or authorize Relay publication/fleet rollout.

### Roadmap-to-issue handoff

- A step is complete only when its exit criteria and required evidence are satisfied; commit count
  never determines progress.
- Ready steps without an issue are candidates for the private, duplicate-aware
  roadmap.issue-plan.json dry run. Planned steps remain preview-only unless a reviewer explicitly
  opts them in with issue_policy: propose.
- Issue creation or reconciliation requires human approval or an explicitly authorized Pace
  operation and returns issue references through a reviewable roadmap pull request.
- Pull requests and commits should include Roadmap-Step: <ID>; historical evidence may be linked
  through existing issue and pull-request relationships.
- Public rendering uses only allowlisted build-time evidence and never places a GitHub token or
  private issue plan in the browser artifact.

<!-- END ROADMAP EXECUTION SNAPSHOT -->

## Strategic context

This roadmap describes capability evolution, not promised dates or an issue queue. Sequence follows
architecture dependencies and may change when evidence or risk changes.

## Phase 1: Define profile and finding contracts

**Outcome:** A bounded capability advances from documented intent to validated, independently usable
behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Phase 2: Extract the proven Empathy implementation

**Outcome:** A bounded capability advances from documented intent to validated, independently usable
behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Phase 3: Publish reusable releases

**Outcome:** A bounded capability advances from documented intent to validated, independently usable
behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Phase 4: Integrate organization conformance and reporting

**Outcome:** A bounded capability advances from documented intent to validated, independently usable
behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Cross-cutting tracks

- Security, privacy, accessibility, licensing, and provenance.
- Documentation, architecture portals, examples, and onboarding.
- Packaging, release, compatibility, and self-hosting.
- Organization integration through explicit contracts.
- Observatory evidence and Pace conformance when those systems exist.

## Deferred direction

Optional managed services, enterprise controls, marketplaces, and the conversational organization
compiler remain later architecture work. Current choices should preserve portability and avoid
foreclosing them.

## Evidence and uncertainty

- **Observed:** The repository README establishes the intended boundary as a universal linting
  platform and extensible MegaLinter wrapper for consistent repository quality; significant
  implementation remains incomplete.
- **Decided for this draft:** The repository owns the bounded concern described here and
  participates through versioned contracts.
- **Proposed:** Target systems and later roadmap phases remain proposals until accepted and
  implemented.
- **Open question:** Which parts of this draft should become active in the first independently
  versioned release?
