# Current-v1 release capability acceptance

This ledger reconciles [issue #29](https://github.com/egohygiene/egolint/issues/29) against the
source candidate at `4e7cc53491a27da5b361ca7ff56568d05fc07701`, the verified final head of
[draft PR #110](https://github.com/egohygiene/egolint/pull/110). It records source and local evidence
for review. Issues #29, #68 and #69 remain open; their original acceptance criteria are preserved.
The current publisher migration, final-head hosted qualification and maintainer acceptance remain
gates. No release, deployment or registry publication is established by this ledger.

The [capability guide](repository-release-validation.md) owns supported checks and report semantics;
the [release operator guide](releasing.md) owns the existing repository adapter. This ledger connects
their evidence to the original work without replacing either guide or the upstream contracts.

## Original acceptance criteria

| Original #29 criterion                                                                                          | Source evidence available for review                                                                                                                                                                                  | Acceptance still required                                                                                                                  |
| --------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Fixtures cover Rust, Python, npm, container, site, publication, workspace, contract-only and archived profiles. | Checkpoint #68 has nine authored profiles and 54 compact focused projections covering six states, plus version drift, entrypoint and no-runtime controls. The repaired #69 archive replays this matrix.               | Maintainer acceptance of draft PR #109; final-head hosted qualification remains deferred.                                                  |
| The validator never claims that an external registry or deployment occurred.                                    | The focused report retains `network_access: not_performed` and `external_publication: not_verified`; native fixtures also check finding, tool and SARIF agreement. This criterion was already checked in #29.         | Retain this boundary in every consumer and any later report change.                                                                        |
| The rule supports advisory-to-required rollout without false conformance.                                       | The existing resolver and matrix preserve separate semantic/check state, effective severity, enforcement and exit. Self-adoption is advisory with a failed workflow check. This criterion was already checked in #29. | Retain the actual failure; process exit 0 must not become a conformance claim.                                                             |
| EgoLint dogfoods the profile.                                                                                   | Checkpoint #69 activates the real incubating declaration and exercises actual source bytes through the public native CLI. Six release checks pass; `manual_workflow` remains failed/advisory.                         | Maintainer acceptance of draft PR #110, the separately owned publisher migration and deferred hosted/platform qualification.               |
| Every checkpoint issue is complete and every checkpoint PR has green validation.                                | Checkpoints #65–#67 are closed and merged, with exact-head historical green runs below. #68/#69 are draft source/local-validation candidates.                                                                         | This criterion is **not satisfied**. New candidates have no claimed hosted green result; their reviews and full acceptance remain pending. |

## Checkpoint and correction evidence

| Work                                                                                    | Observed disposition                                                          | Exact evaluated source                     | Evidence boundary                                                                                                                                                                                  |
| --------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- | ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [#65](https://github.com/egohygiene/egolint/issues/65): immutable inputs                | Closed; [PR #64](https://github.com/egohygiene/egolint/pull/64) merged        | `b8de3ed2636e1cb7b8e81e9c8737f555ddb31a0a` | Historical [CI](https://github.com/egohygiene/egolint/actions/runs/35465366245) and [Dogfood](https://github.com/egohygiene/egolint/actions/runs/35465366258) completed successfully on this head. |
| [#66](https://github.com/egohygiene/egolint/issues/66): applicability/reports           | Closed; [PR #70](https://github.com/egohygiene/egolint/pull/70) merged        | `e00a6b87d15b244418ed8ced200c613994d3d3dc` | Historical [CI](https://github.com/egohygiene/egolint/actions/runs/35469058347) and [Dogfood](https://github.com/egohygiene/egolint/actions/runs/35469058350) completed successfully on this head. |
| [#67](https://github.com/egohygiene/egolint/issues/67): mechanical checks               | Closed; [PR #71](https://github.com/egohygiene/egolint/pull/71) merged        | `9f2073f39df3851584be8956a2b27f7d506739b5` | Historical [CI](https://github.com/egohygiene/egolint/actions/runs/35473934916) and [Dogfood](https://github.com/egohygiene/egolint/actions/runs/35473934967) completed successfully on this head. |
| [#91](https://github.com/egohygiene/egolint/issues/91): exact static handoff correction | Open; [PR #108](https://github.com/egohygiene/egolint/pull/108) draft         | `36a3163b4017fc2d9b7ece873e896e6dca8fd96b` | Direct/quoted controls and 32 Task-source cases exercise actual CLI/focused/run/SARIF behavior. Commands are inspected, never dispatched.                                                          |
| [#68](https://github.com/egohygiene/egolint/issues/68): fixture matrix                  | Open; [PR #109](https://github.com/egohygiene/egolint/pull/109) draft on #108 | `4e21013c0f1f0e9998cb54775b80da21d890f339` | Nine-profile/six-state source matrix; the first crate archive omitted the nested Rust fixture despite its include glob.                                                                            |
| [#69](https://github.com/egohygiene/egolint/issues/69): self-dogfood                    | Open; [PR #110](https://github.com/egohygiene/egolint/pull/110) draft on #109 | `4e7cc53491a27da5b361ca7ff56568d05fc07701` | Source/local review candidate, including the archive repair and unpacked replay below. Hosted and full acceptance remain pending.                                                                  |

Historical green runs qualify their recorded heads only. The new draft stack uses `[skip ci]`;
hosted Actions, full-image and native non-Linux qualification are deferred. A merged historical PR,
a local passing suite and a completed external release are different evidence. PR #64 is historical;
the old instruction to move it out of draft does not apply to these new focused candidates.

## Packaging qualification receipt

The initial #68 include glob and a successful crate build did not establish fixture completeness.
Actual unpacked execution in #69 exposed Cargo excluding the Rust fixture directory because it
contained a nested package manifest. That initial result remains a failed qualification attempt.

Checkpoint #69 stores the same authored manifest bytes as
[`rust/Cargo.toml.fixture`](../tests/fixtures/repository-release/rust/Cargo.toml.fixture). Native and
Python materializers restore `Cargo.toml` only inside disposable consumers; declarations and expected
reports retain the real consumer authority path. The predecessor manifest and inert fixture have
identical Git blob identity `841ddb907b6637e791a23737db4f82d59543f8a4`.

At the exact #69 head above, a 215-file crate built and verified with locked cached inputs. Archive
inspection found every tracked release fixture, and actual unpacked execution passed two self tests,
seven matrix tests and five planner CLI tests. The final archive also contained the exact final
handoff and fixture README bytes. This is stronger than inventory inspection, while remaining local
package qualification without publication.

The same #69 source passed formatting, strict Clippy, 219 Rust tests with zero ignored, 80 Python tests
and 23 schema comparisons. Its draft PR records the precise commands, review repairs and remaining
limits. Those results do not create a hosted pass for this later documentation candidate.

## Separate self-consumer verdicts

| Surface                     | Actual #69 source result                                    | Meaning                                                                                                                  |
| --------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| Process                     | Exit 0                                                      | Effective advisory enforcement is nonblocking.                                                                           |
| Release report              | `advisory`; six passed, `manual_workflow` failed            | The tag-push publisher remains unmigrated. Manual dispatch is an existing non-publishing rehearsal.                      |
| Continuity report           | Valid with the expected unreleased-contract observe warning | Local structure/freshness/Git evidence passed for the declared comparison; released upstream conformance is not claimed. |
| Broader Intelligence report | `invalid`, nonblocking; seven diagnostics                   | Unknown ADR evidence and six pre-existing roadmap state-format warnings remain under #101.                               |

Source release-profile exercise does not establish repository-wide conformance. The current-v1
manual-only publisher migration remains separately reviewed work under
[issue #5](https://github.com/egohygiene/egolint/issues/5). The broader documentation findings remain
under [issue #101](https://github.com/egohygiene/egolint/issues/101). Neither is hidden by the local
process result or folded into a false completed parent state.

## Operator and consumer handoff

For bounded preparation, the known implementation pin is
`4e7cc53491a27da5b361ca7ff56568d05fc07701` from draft PR #110. It includes the #91 correction,
#68 matrix and #69 archive repair. Use a clean disposable producer checkout and verify that exact
commit before building or replaying controls. The pinned Rust toolchain and lockfiles remain part of
the source contract:

```sh
producer_revision="4e7cc53491a27da5b361ca7ff56568d05fc07701"
test "$(git rev-parse --verify HEAD)" = "$producer_revision"
python3 scripts/validate_repository_release_sources.py
cargo test --locked --test "repository_release_matrix" --test "repository_release_dogfood"
python3 -m unittest discover --start-directory "tests" --pattern "test_repository_release_plan.py"
cargo build --locked --bin "egolint"
```

The selected Aether schema and Hygiene applicability profile remain the exact inputs in the
[source lock](../.config/rules/repository-release-sources.v1.json). Their reviewed bytes do not acquire
new authority from a local check. Current self-adoption and package qualification are described
above; [the capability guide](repository-release-validation.md) owns the detailed commands and
supported grammar.

A final #29 documentation candidate may instead be pinned by the full commit recorded in its draft
PR handoff. Verify its runtime-source difference against the implementation pin and qualify that
exact checkout. Record the actual producer commit and executable SHA-256 in the consumer's evidence;
the containing ledger does not need to embed its own future commit identifier. A source pin is not a
published binary, container digest or accepted release.

[Mantle #26](https://github.com/egohygiene/mantle/issues/26) can consume the native producer for bounded
current-v1 source preparation while its full-outcome gates remain open. Select the reviewed Mantle
checkout as `MANTLE_WORKSPACE` and the qualified native executable as `EGOLINT_RELEASE_PRODUCER`.
Invoke that executable directly; retain its actual path/digest and any custom Cargo target directory
in the receipt:

```sh
"${EGOLINT_RELEASE_PRODUCER:?select the qualified native executable}" \
  --workspace "${MANTLE_WORKSPACE:?select the reviewed Mantle checkout}" validate
```

This native source inspection does not execute a release Task or dispatch a workflow. Preserve any
existing consumer configuration; supply an explicit repository-context input only when it is owned
and adopted by that consumer. EgoLint's `.config/dogfood/repository-context.toml` identifies EgoLint
and must not be copied into Mantle as an authority or identity assertion. The caller remains
responsible for binding supplied repository identity and captured source to the authenticated
checkout; offline conformance does not establish GitHub ownership or live publication state.

Keep a finite consumer receipt:

| Receipt field | Evidence to retain                                                                                            |
| ------------- | ------------------------------------------------------------------------------------------------------------- |
| Producer      | Full source commit, executable SHA-256, build command and pinned toolchain.                                   |
| Consumer      | Exact reviewed commit/tree, any intentional working-tree difference and consumer-owned declaration/context.   |
| Inputs        | Selected source-lock identities and the bytes/digests actually inspected.                                     |
| Invocation    | Complete native argv and effective consumer configuration.                                                    |
| Results       | Process exit, focused release/check state, enforcement, tool counts and normalized findings/SARIF.            |
| Limits        | Unavailable/external evidence, platform and hosted omissions, pending review, publication and adoption gates. |

Read `.reports/egolint/repository-release.json`, `run.json` and `egolint.sarif` in the consumer's report
boundary separately from process exit. Retain every failed, advisory, unavailable or externally owned
check. Mantle's installation identity, semver compatibility, non-overwriting publication path and
supported-platform evidence remain its own acceptance responsibilities. The current EgoLint
self-adoption gap and remaining producer gates remain visible rather than transferring a completed
producer or fleet-conformance claim to the consumer.

## Remaining acceptance gates

The review order is #91 → #68 → #69 → this parent #29 ledger. Reconcile each continuity snapshot
when its candidate is accepted. Keep all three new release-chain PRs and this ledger in draft until
the maintainer chooses their disposition; no issue checklist or state is changed here.

Full #69/#29 acceptance still needs the separately reviewed publisher migration, actual final-head
hosted/full-image/non-Linux evidence and maintainer acceptance of the stack. Signed immutable product
release, registry acceptance and consumer adoption require their own evidence under their owning
issues. Successor release-model issue #76 does not replace this current-v1 work or promote the
selected upstream authority.
