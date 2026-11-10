# Release dependency and M1 handoff

Observed 2026-11-10. This candidate is stacked on locally qualified
[PR #112](https://github.com/egohygiene/egolint/pull/112) at
`ef636747632131ccd68d93c7e5e0abf4ad9f0cee`. Observed main remains
`f14f7fd3ab0ce9e87c367352fcb3af11a32eda59`. Integrate #112 first, then retarget this child to main.
Neither PR was merged by this work.

## Scope and ownership

| Issue       | Current source evidence                                                                     | Remaining boundary                                                                                                                              |
| ----------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| #35/#36/#37 | Qualified parent capability selection, enforcement and pinned advisory structural controls. | M1 review and merge; documented provider/platform limits remain.                                                                                |
| #90         | Delivered #103 grammar repair reverified by all 14 valid/malformed/environment tests.       | No build, registry or publication claim follows from source validation.                                                                         |
| #101        | Current ADR/platform/architecture/roadmap guidance and continuity are reconciled here.      | Historical ADR coverage remains explicitly unknown; proposed policy is not promoted.                                                            |
| #5          | Manual publisher source is implemented and locally tested.                                  | Protected configuration, current-head provider/platform qualification, signed immutable publication and pinned Empathy plus one other consumer. |

Former #68/#69 tracking is consolidated into #29. Their complete acceptance and historical receipts
remain in that issue and the [release acceptance ledger](repository-release-acceptance.md).
Current-v1 source conformance, successor #76 authority and actual #5 release/adoption are separate
outcomes. Roadmap #42 owns the eight-milestone execution order.

## Manual publication boundary

The Release workflow exposes only `workflow_dispatch`; rehearsal is the default. An explicit publish
request must match Cargo version, full dispatch/checkout revision, freshly observed main and the
existing matching immutable tag. It requires authenticated successful rehearsal metadata from the
same repository, workflow and source, and cannot approve itself. Seven real disposable-Git request
controls preserve rejection of mismatched, incomplete or ambiguous evidence.

Candidate jobs retain read-only contents/Actions access. Protected signing, attestation, quarantine
attempts, exact digest verification, platform smoke, write-once promotion and release announcement
remain in their existing separate jobs. These static/local controls do not prove production
permissions or registry behavior. No workflow was dispatched, tag created, artifact signed or
product published here.

## Current source guidance

Current ADR policy remains accepted 1.2.0 at Hygiene `a05b14f8b9e2772124f879f818e09a911e996ad8`;
schema SHA-256 is `725a7cf58cd0b0454d4d481e8ef72815f29ecca80a5b44da5ce48f7ec6e6b8c4`. The supported
immutable ordinary-consumer 1.1.0 alternative at `c589587395750cd1c79c6fa0bef010189c547249` retains
digest `491eae26e498ee311bb37d5eff23a51d682eb687196c80a3df10ac1ce98e04d4`. Catalog/vendor bytes are
preserved.

CLI products select AMD64 and ARM64; the full engine remains AMD64-only. Upstream-dependent #45
expansion does not block that first-release boundary. SYSTEM/ARCHITECTURE identify actual source
owners and distinguish implemented, in-review and deferred work. ROADMAP follows M1–M8 and separates
historical checkpoint text from the live issue tracker. Intelligence retains explicit unknown ADR
adoption rather than inventing complete coverage.

## Local qualification

Rust 1.85.1 passes **267 tests across 18 suites**, strict Clippy, formatting, all 27 generated
native schema comparisons, policy/integration contracts and verified Cargo packaging. All **95
Python tests** pass, including 14 release-input and seven request controls. The actual
non-publishing Bash schema step extracted from the Release workflow passes locally.

All 94 JavaScript adapter/package controls, five commit/config controls and the actual
architecture/package self-consumer pass. Matching Mypy 1.19.1, Bandit 1.9.4, markdownlint 0.49.1,
Prettier 3.9.6 and selected Ruff 0.16.2 checks pass. The real ast-grep 0.51.1 controls produce
8/256/1,024 expected positives and zero negative matches across 16/512/2,048 files. Root continuity
records the exact schema/format/native comparison result and source acceptance limits.

The former hosted Dogfood failures are retained as historical evidence, while the matching blocking
checks now pass directly. Digest-pinned full-engine pull and original CLI Docker build attempts
exhausted the 32 GiB local vfs storage. No current image success is claimed; only task-created build
cache was cleared. Current-head hosted Actions and native non-Linux qualification remain unrun.

Aggregate-only adapter reports cannot establish reviewed ratchet baselines; complete normalized
evidence and the exact producing executable are required. Selected continuity inputs remain
draft/proposed and observe-only. Historical audits, vendor authorities, dependencies and lockfiles
are unchanged. Broader structural precision/stress, signed publication, consumer adoption and shared
version/changelog automation remain in their owning milestones.

ADR-Ref: egohygiene/egolint#ADR-0001, egohygiene/egolint#ADR-0003. This qualification preserves
existing wrapper and evidence ownership; it changes no human decision status or upstream authority.
