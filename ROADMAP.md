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
updated: 2026-08-24
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
updated: 2026-08-24
-->
## 2026-08-24 execution snapshot

> This evidence-reconciled snapshot is the issue-generation and visual-roadmap handoff. The longer-horizon strategy below remains canonical context; generated HTML, JSON, progress, issue plans, and commit lists are projections.

**Lifecycle:** early alpha, pre-release  
**Current gate:** Turn the red self-dogfood run and its 119 findings into an owned, green baseline through issues #20 and #21.  
**North-star outcome:** A portable, evidence-backed lint platform for organization contracts, roadmaps, repositories, and generated artifacts.

### Visual roadmap publication

**Mode:** `central`  
**Route:** `/roadmap/egolint/`  
**Current publication evidence:** Source and CI reports on GitHub; no public release observed.

Publish the public-safe projection through egohygiene.io at /roadmap/egolint/. This repository owns intent and acceptance evidence; it does not add a second site deployment.

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
status: active
depends_on: [EGL-Q01]
issues: [20, 21]
-->
#### EGL-Q02 — Classify the dogfood findings

**State:** `active`  
**Depends on:** `EGL-Q01`

**Outcome:** Every self-dogfood finding is either fixed, accepted with rationale, or identified as a rule defect.

**Exit criteria:**

- [ ] All 119 observed findings have an owner and disposition.
- [ ] The baseline does not silently suppress unexplained failures.

**Current evidence:**

- Dogfood run 32774944039 failed with 119 findings.
- Issues #20 and #21 track the current gate.

<!-- roadmap-step
id: EGL-Q03
status: blocked
depends_on: [EGL-Q02]
issues: []
-->
#### EGL-Q03 — Make self-dogfood green

**State:** `blocked`  
**Depends on:** `EGL-Q02`

**Outcome:** Egolint passes its own accepted policy and proves rule/report stability.

**Exit criteria:**

- [ ] The default-branch dogfood workflow is green.
- [ ] Known exceptions are explicit, reviewed, and expiring where appropriate.

**Current evidence:**

- The latest audited dogfood run was red.

<!-- roadmap-step
id: EGL-Q04
status: planned
depends_on: [EGL-Q03]
issues: []
-->
#### EGL-Q04 — Version the evidence report contract

**State:** `planned`  
**Depends on:** `EGL-Q03`

**Outcome:** Other tools can consume stable lint findings and link them to roadmap gates.

**Exit criteria:**

- [ ] A versioned machine-readable report schema is documented.
- [ ] A fixture proves backward-compatible parsing.

**Current evidence:**

- A stable cross-tool evidence contract was not observed.

<!-- roadmap-step
id: EGL-Q05
status: planned
depends_on: [EGL-Q03, EGL-Q04]
issues: []
-->
#### EGL-Q05 — Publish and adopt the first release

**State:** `planned`  
**Depends on:** `EGL-Q03`, `EGL-Q04`

**Outcome:** A pinned Egolint release enforces the roadmap contract in representative repositories.

**Exit criteria:**

- [ ] A tagged release is published with immutable artifacts.
- [ ] At least three repositories use the pinned release successfully.

**Current evidence:**

- No release was observed.

<!-- roadmap-step
id: EGL-Q06
status: planned
depends_on: [HYG-Q06, EGL-Q05]
issues: []
-->
#### EGL-Q06 — Validate roadmap execution graphs

**State:** `planned`  
**Depends on:** `HYG-Q06`, `EGL-Q05`

**Outcome:** A pinned Egolint release validates roadmap IDs, states, dependencies, issue references, and completion evidence without network writes.

**Exit criteria:**

- [ ] Valid, invalid, cyclic, stale, and private fixtures are covered.
- [ ] A consumer workflow produces an actionable machine-readable report.

**Current evidence:**

- Roadmap-graph validation is assigned to Egolint by the 2026-08-24 visual-roadmap specification.

### Roadmap-to-issue handoff

- A step is complete only when its exit criteria and required evidence are satisfied; commit count never determines progress.
- Ready steps without an issue are candidates for the private, duplicate-aware roadmap.issue-plan.json dry run. Planned steps remain preview-only unless a reviewer explicitly opts them in with issue_policy: propose.
- Issue creation or reconciliation requires human approval or an explicitly authorized Pace operation and returns issue references through a reviewable roadmap pull request.
- Pull requests and commits should include Roadmap-Step: <ID>; historical evidence may be linked through existing issue and pull-request relationships.
- Public rendering uses only allowlisted build-time evidence and never places a GitHub token or private issue plan in the browser artifact.

<!-- END ROADMAP EXECUTION SNAPSHOT -->

## Strategic context

This roadmap describes capability evolution, not promised dates or an issue queue. Sequence follows architecture dependencies and may change when evidence or risk changes.

## Phase 1: Define profile and finding contracts

**Outcome:** A bounded capability advances from documented intent to validated, independently usable behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Phase 2: Extract the proven Empathy implementation

**Outcome:** A bounded capability advances from documented intent to validated, independently usable behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Phase 3: Publish reusable releases

**Outcome:** A bounded capability advances from documented intent to validated, independently usable behavior.

**Exit signals:**

- The owning contract and acceptance criteria are versioned.
- Implementation and documentation agree.
- Relevant tests and safety checks pass.
- Downstream consumers and migration impact are understood.
- Remaining uncertainty is visible.

## Phase 4: Integrate organization conformance and reporting

**Outcome:** A bounded capability advances from documented intent to validated, independently usable behavior.

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

Optional managed services, enterprise controls, marketplaces, and the conversational organization compiler remain later architecture work. Current choices should preserve portability and avoid foreclosing them.

## Evidence and uncertainty

- **Observed:** The repository README establishes the intended boundary as a universal linting platform and extensible MegaLinter wrapper for consistent repository quality; significant implementation remains incomplete.
- **Decided for this draft:** The repository owns the bounded concern described here and participates through versioned contracts.
- **Proposed:** Target systems and later roadmap phases remain proposals until accepted and implemented.
- **Open question:** Which parts of this draft should become active in the first independently versioned release?
