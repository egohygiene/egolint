# Repository release validation

EgoLint validates repository release declarations against immutable, separately owned policy
inputs. The capability remains offline and evidence-based: it does not publish releases, mutate
workflows, or infer conformance from the existence of a tag alone.

## Ownership boundary

| Concern                             | Owner             | EgoLint responsibility                                                                                    |
| ----------------------------------- | ----------------- | --------------------------------------------------------------------------------------------------------- |
| Normative release declaration       | Aether            | Validate the declared repository-release document against the accepted schema.                            |
| Applicability and required evidence | Hygiene           | Apply the accepted profile and lifecycle policy without broadening it locally.                            |
| Repository inspection and findings  | EgoLint           | Inspect declarations, changelogs, version sources, workflows, and task entrypoints; emit stable evidence. |
| Evidence retention                  | Relay             | Preserve accepted reports and release-gate evidence outside this repository.                              |
| Publication adapters                | Owning repository | Build, sign, attest, and publish artifacts through explicitly configured release workflows.               |

Issue 35 retains general capability discovery and execution-plan orchestration. Repository release
applicability itself is now a universal native capability: `lint`, `validate`, and `fix` inspect the
policy-selected declaration path automatically and emit a focused report even when the declaration
is absent. An authorized planner can select an explicit adoption state; otherwise EgoLint derives
rollout from the declaration lifecycle and the accepted Hygiene profile.

## Accepted source revisions

No GitHub Release or tag existed for either accepted input when this boundary was reviewed. The
merged dependency commits are therefore the immutable source revisions for this implementation.
`.config/rules/repository-release-sources.v1.json` records the source path, vendored path, Git blob
identity, and SHA-256 digest for each input.

| Input                                         | Accepted revision                          | Vendored file                                            | SHA-256                                                            |
| --------------------------------------------- | ------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------------ |
| `egohygiene/aether` repository-release schema | `8a2a3d08f3aa9da3847bd5277843506ab855192e` | `vendor/aether/aether.repository-release.v1.schema.json` | `8431ea7651336aa7695aee5b9aedce536b1e7c76c3cb15927f64a6206f9cc1f5` |
| `egohygiene/hygiene` applicability profile    | `28f9d6c7519d820644572634ba4476614f418d83` | `vendor/hygiene/repository-release-policy.v1.json`       | `20030513c311416c5130a15af897590892025b55cfd79d210be362d47748063f` |

Run the offline source check with:

```sh
python "scripts/validate_repository_release_sources.py"
```

The check rejects unlocked sources, path traversal, symlinks, byte drift, incompatible Aether
references, and profile/lifecycle drift between the two contracts.

## Applicability and report states

The native resolver does not need a network connection and never treats an external declaration as
proof of publication. It composes slot defaults with profile and lifecycle overrides, then applies
the repository adoption state. Required slots become advisory for advisory or exempt rollout;
explicit `not-applicable` makes every slot not applicable even when no release declaration exists.

`egolint.repository-release-report/v1` is generated from closed Rust types and written to
`.reports/egolint/repository-release.json`:

| State            | Meaning                                                                                  |
| ---------------- | ---------------------------------------------------------------------------------------- |
| `compliant`      | Complete local checks cover every applicable slot without failures or unavailable input. |
| `advisory`       | Hygiene rollout or an authorized exemption keeps applicable checks nonblocking.          |
| `unavailable`    | The declaration, declared evidence, or complete local validation is unavailable.         |
| `external`       | The declaration assigns evidence to another owner; reachability was not checked.         |
| `invalid`        | The declaration cannot establish trusted applicability or a completed check failed.      |
| `not_applicable` | An authorized planner explicitly selected non-applicability.                             |

Checkpoint 2 resolves applicability only, so ordinary required repositories remain `unavailable`
until checkpoint 3 supplies complete check coverage. This prevents an empty validator from
reporting false conformance. Use `egolint schema repository-release-report` to emit the exact
machine contract. Human-readable commands print the same state and the non-publication boundary.

## Checkpoint plan

1. Pin and verify the immutable Aether and Hygiene inputs; reconcile the preceding roadmap state.
2. Resolve native release policy and applicability, then define the focused report contract.
3. Validate declarations, changelog policy, version authority, release workflows, and task
   entrypoints.
4. Prove the boundary with Rust, Python, npm, container, site, publication, workspace,
   contract-only, archived, advisory, external, unavailable, and invalid fixtures.
5. Dogfood the completed capability, document operations and limits, and perform final CI polish.

Checkpoint 1 landed in PR 64. Each remaining checkpoint uses its own focused pull request and stops
for maintainer review before merge.
