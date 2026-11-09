# Release dependency source handoff

Observed 2026-11-09. This candidate is stacked on
[PR #112](https://github.com/egohygiene/egolint/pull/112), exact review base
`3178561fc500062d1dde2a637fa85e59efb290ab`. Observed main remains
`f14f7fd3ab0ce9e87c367352fcb3af11a32eda59`. Review/merge #112 first, then
retarget this candidate to main after that accepted integration. No PR has been
merged by this work.

## Dependency disposition

| Issue                                                    | Source disposition                                                                                                                                              | Remaining acceptance                                                                                                                                 |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| [#5](https://github.com/egohygiene/egolint/issues/5)     | Manual publisher migration implemented in this candidate; scanner ownership, compact debt and signing/promotion scaffolding already exist.                      | Hosted release qualification, protected environment/configuration, signed immutable publication and digest-pinned Empathy plus non-Empathy adoption. |
| [#90](https://github.com/egohygiene/egolint/issues/90)   | Typed release-input repair already merged through [PR #103](https://github.com/egohygiene/egolint/pull/103); actual valid/malformed/environment controls rerun. | Original issue remains open for acceptance; no duplicate validator or closure is introduced.                                                         |
| [#101](https://github.com/egohygiene/egolint/issues/101) | ADR tuples/digests, product platforms, source architecture, current roadmap and continuity reconciled in this candidate.                                        | Review the source correction; unknown ADR coverage and historical evidence remain explicit.                                                          |

#5's historical prerequisites EgoLint #2, EgoLint #4 and Relay #4 are closed.
#29's original schema/profile dependencies are delivered; #68/#69 source is
already merged. Their remaining acceptance is retained in the
[release ledger](repository-release-acceptance.md), rather than duplicated.
All open issue states are preserved.

## Manual publication boundary

The Release workflow now has only `workflow_dispatch`. The default operation
rehearses. An explicit publish request must match the Cargo version, full
dispatch/checkout revision and freshly fetched main. It also needs the existing
matching version tag and authenticated Actions metadata proving an earlier
successful rehearsal of the same repository, workflow and source.

The repository-owned validator rejects another workflow/repository, changed
source, tag/branch mismatch, failed/incomplete rehearsal, ambiguous version,
malformed metadata and a run approving itself. The candidate job has read-only
contents/Actions access; signing, attestations, image writes and announcement
retain their existing protected release environment and dependency chain.
Immutable digest checks, quarantine attempts, supported-platform smoke and
write-once version promotion remain intact.

The source self-consumer passes all seven release checks while retaining
incubating/advisory rollout and `external_publication: not_verified`. A hostile
copy restores tag automation and verifies the failed advisory finding returns.
The historical #69 receipt still describes its exact older six-pass/one-failure
source; that evidence is not rewritten.

The workflow verifies every generated native schema, including the additional
contracts from #112. Four authored JavaScript schema files remain with their
Node owners. Operator commands and the exact rehearsal/publication inputs are
in [release guidance](releasing.md). No Release workflow was dispatched, no
repository release tag created, and no artifact signed or published here.

## Source guidance correction

- Current ADR policy is accepted 1.2.0 at Hygiene
  `a05b14f8b9e2772124f879f818e09a911e996ad8`. Its schema SHA-256 is
  `725a7cf58cd0b0454d4d481e8ef72815f29ecca80a5b44da5ce48f7ec6e6b8c4`.
  The exact accepted ordinary-consumer 1.1.0 alternative at
  `c589587395750cd1c79c6fa0bef010189c547249` retains schema digest
  `491eae26e498ee311bb37d5eff23a51d682eb687196c80a3df10ac1ce98e04d4`.
  Both immutable Git objects and the current catalog/vendor bytes were checked.
- CLI OCI products select AMD64 and ARM64; the full MegaLinter engine selects
  AMD64 only. Future full-engine ARM64 remains #45.
- SYSTEM/ARCHITECTURE now identify actual source owners and implemented,
  in-review and deferred boundaries. They distinguish a built package from a
  published distribution.
- Roadmap steps EGL-Q07/EGL-Q11 reflect closed delivered work. The six combined
  state/dependency line warnings are corrected in source. Native Intelligence
  now reports only the explicit unknown-ADR adoption warning and remains
  `incomplete`; no rule or severity is disabled.
- EgoLint #73/#81 and Relay #144 are closed. Relay's exact historical adoption
  remains attributed to its own producer pin, separate from these candidates.

## Validation and limits

The candidate is checked with Rust 1.85.1:

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo package --locked --allow-dirty
task schemas:check
python3 -m unittest discover --start-directory tests --pattern 'test_*.py'
python3 scripts/validate_repository_release_sources.py
python3 scripts/validate_release_inputs.py
```

The final exact test count, packaging and continuity outcomes are recorded in
the root checkpoint. All **95 Python tests** pass, including real disposable-Git
release-request controls and the retained typed input grammar matrix. The actual
Bash schema step extracted from the release workflow also passes. Selected Ruff,
Biome, Prettier and whitespace checks pass.

The JavaScript protocol lookup repair is inherited unchanged from #112. Its
94 local controls and current hosted package-quality check pass; this candidate
does not alter those adapter/test bytes. On #112's reviewed base, hosted CI,
architecture and identity also pass. Its full-image Dogfood reaches analysis
and fails with Markdown, Python analyzer and YAML formatting findings. That
result is retained; no full-image green or release readiness is claimed.

Hosted validation for this new candidate, native non-Linux release acceptance,
actual signing/registry behavior and consumer adoption remain unqualified.
Historical audit artifacts, vendored authorities, dependencies and lockfiles
are unchanged. Version/changelog promotion and protected publication are
separate review actions, not outcomes inferred from these source checks.
