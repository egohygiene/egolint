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

Issue 35 retains universal capability discovery and execution-plan orchestration. Repository release
validation is designed to be available to every repository while returning explicit
`not-applicable`, `unavailable`, or advisory evidence when a rule cannot meaningfully run.

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

## Checkpoint plan

1. Pin and verify the immutable Aether and Hygiene inputs; reconcile the preceding roadmap state.
2. Resolve native release policy and applicability, then define the focused report contract.
3. Validate declarations, changelog policy, version authority, release workflows, and task
   entrypoints.
4. Prove the boundary with Rust, Python, npm, container, site, publication, workspace,
   contract-only, archived, advisory, external, unavailable, and invalid fixtures.
5. Dogfood the completed capability, document operations and limits, and perform final CI polish.

Each checkpoint lands on the same draft pull request. Maintainer merge remains blocked until the
final checkpoint and review.
